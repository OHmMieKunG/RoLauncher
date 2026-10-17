use super::*;
use crate::{control_policy::Deadline, resource::*};

impl Engine {
    #[cfg(windows)]
    fn activity_eligible(&self, account: &Account) -> bool {
        if self.is_shutdown() || self.session_suspended() {
            return false;
        }
        let state = self.0.state.lock().unwrap();
        !state.anti_afk_paused
            && state.database.accounts.iter().any(|a| {
                let current = &a.account;
                current.id == account.id
                    && current.process == account.process
                    && current.generation == account.generation
                    && current.client == account.client
                    && current.resource == account.resource
                    && current.anti_afk == account.anti_afk
                    && activity_allowed(current)
            })
    }
    pub fn set_anti_afk_paused(&self, paused: bool) {
        let mut state = self.0.state.lock().unwrap();
        state.anti_afk_paused = paused;
        let _ = self.0.events.send(snapshot(&state));
    }
    pub(super) fn session_suspended(&self) -> bool {
        self.0.session_suspended.load(Ordering::Acquire) || crate::controls::suspended()
    }
    pub(super) fn spawn_automation(&self) {
        let mut worker = self.0.automation.lock().unwrap();
        if worker.is_some() {
            return;
        }
        let engine = self.clone();
        *worker = Some(std::thread::spawn(move || engine.automation_loop()));
    }
    fn automation_loop(&self) {
        #[cfg(windows)]
        let mut environment = crate::controls::Environment::new();
        #[cfg(windows)]
        let mut managed: HashMap<String, crate::controls::Managed> = HashMap::new();
        #[cfg(windows)]
        let mut retiring: Vec<crate::controls::Managed> = Vec::new();
        #[cfg(windows)]
        let mut gpu = crate::gpu_collector::GpuCollector::new();
        #[cfg(windows)]
        let preexisting: HashMap<_, _> = self
            .snapshot()
            .accounts
            .into_iter()
            .filter_map(|a| a.process.map(|p| (a.id, p)))
            .collect();
        let mut deadlines: HashMap<String, (ProcessIdentity, AntiAfkSettings, Deadline)> =
            HashMap::new();
        let mut last_tick = Instant::now();
        let mut sample_at = Instant::now();
        while !self.is_shutdown() {
            #[cfg(windows)]
            for control in managed.values_mut().chain(retiring.iter_mut()) {
                let _ = control.release_pending();
            }
            #[cfg(windows)]
            if environment.pump() {
                let paused = self.0.state.lock().unwrap().anti_afk_paused;
                self.set_anti_afk_paused(!paused);
            }
            let now = Instant::now();
            let suspended = crate::controls::suspended();
            let changed = self.0.session_suspended.swap(suspended, Ordering::AcqRel) != suspended;
            if changed || now.duration_since(last_tick) > Duration::from_secs(3) {
                deadlines.clear();
            }
            if changed {
                let mut state = self.0.state.lock().unwrap();
                state.session_suspended = suspended;
                let _ = self.0.events.send(snapshot(&state));
            }
            last_tick = now;
            if suspended || self.0.state.lock().unwrap().anti_afk_paused {
                deadlines.clear();
            }
            #[cfg(windows)]
            {
                for (id, control) in &mut managed {
                    if !control.has_pending_window() && !control.has_activity() {
                        continue;
                    }
                    let current = {
                        let state = self.0.state.lock().unwrap();
                        state
                            .database
                            .accounts
                            .iter()
                            .find(|a| a.account.id == *id)
                            .map(|a| a.account.clone())
                    };
                    let owned = current.as_ref().is_some_and(|a| {
                        a.process.as_ref() == Some(control.identity())
                            && a.generation == control.identity().generation
                            && control.matches_preferences(&a.resource, &a.client)
                    });
                    let activity_changed = control.has_activity();
                    if activity_changed {
                        if owned {
                            if let Some(submitted) = control.advance_activity(|| {
                                self.activity_eligible(current.as_ref().unwrap())
                            }) && let Some(entry) = deadlines.get_mut(id)
                            {
                                entry.2.reschedule(
                                    Instant::now(),
                                    Duration::from_secs(if submitted {
                                        ANTI_AFK_SECONDS
                                    } else {
                                        60
                                    }),
                                );
                            }
                        } else {
                            control.cancel_activity(false);
                        }
                    }
                    if owned && !self.is_shutdown() && !suspended {
                        // The retained verified handle pins process identity; no WMI queries on this fast path.
                        control.advance_pending();
                    } else {
                        control.cancel_window_transition();
                    }
                    if activity_changed {
                        let mut state = self.0.state.lock().unwrap();
                        if let Some(result) = state
                            .runtime_controls
                            .get_mut(id)
                            .filter(|r| r.identity.as_ref() == Some(control.identity()))
                        {
                            result.client_mode = None;
                            control.observed_mode(result);
                            control.activity_status(result);
                        }
                        let _ = self.0.events.send(snapshot(&state));
                    }
                }
            }
            if now >= sample_at && !suspended {
                sample_at = now + Duration::from_secs(2);
                let view = self.snapshot();
                #[cfg(windows)]
                if view
                    .accounts
                    .iter()
                    .any(|a| a.process.is_some() && a.client.monitoring)
                {
                    gpu.request();
                }
                #[cfg(windows)]
                {
                    let outdated: Vec<_> = managed
                        .iter()
                        .filter(|(id, control)| {
                            !view.accounts.iter().any(|a| {
                                a.id == **id
                                    && a.process.as_ref() == Some(control.identity())
                                    && a.generation == control.identity().generation
                            })
                        })
                        .map(|(id, _)| id.clone())
                        .collect();
                    for id in outdated {
                        retiring.push(managed.remove(&id).unwrap());
                    }
                    retiring.retain_mut(|control| control.restore().is_err());
                }
                deadlines.retain(|id, _| {
                    view.accounts
                        .iter()
                        .any(|a| a.id == *id && a.process.is_some())
                });
                let mut runtime = BTreeMap::new();
                #[cfg(windows)]
                let mut activity_busy = managed.values().any(|c| c.has_activity());
                for account in &view.accounts {
                    let Some(identity) = &account.process else {
                        continue;
                    };
                    if identity.generation != account.generation {
                        continue;
                    }
                    let mut result = RuntimeControls {
                        identity: Some(identity.clone()),
                        sampled_at: Some(Utc::now()),
                        roblox_acceptance: "unverified",
                        ..Default::default()
                    };
                    #[cfg(windows)]
                    {
                        // Verify outside the state lock, then recheck account ownership before OS actions.
                        let control = managed
                            .entry(account.id.clone())
                            .or_insert_with(|| crate::controls::Managed::new(identity.clone()));
                        let verified = control.verified_process().and_then(|process| {
                            control.attach_journal_once(
                                self.data_directory(),
                                &account.id,
                                preexisting.get(&account.id) == Some(identity),
                            )?;
                            Ok(process)
                        });
                        match verified {
                            Ok(process) => {
                                let state = self.0.state.lock().unwrap();
                                let current = state
                                    .database
                                    .accounts
                                    .iter()
                                    .find(|a| a.account.id == account.id)
                                    .map(|a| &a.account);
                                if self.is_shutdown()
                                    || current.is_none_or(|a| {
                                        a.generation != account.generation
                                            || a.process.as_ref() != Some(identity)
                                    })
                                {
                                    continue;
                                }
                                let account = current.unwrap().clone();
                                let paused = state.anti_afk_paused;
                                drop(state);
                                if account.client.monitoring {
                                    control.sample(&process, &mut result);
                                }
                                result.resource_result =
                                    control.apply(&process, &account.resource, &account.client);
                                let eligible = !paused
                                    && !self.session_suspended()
                                    && activity_allowed(&account);
                                let entry =
                                    deadlines.entry(account.id.clone()).or_insert_with(|| {
                                        (
                                            identity.clone(),
                                            account.anti_afk.clone(),
                                            Deadline::default(),
                                        )
                                    });
                                if entry.0 != *identity || entry.1 != account.anti_afk {
                                    *entry = (
                                        identity.clone(),
                                        account.anti_afk.clone(),
                                        Deadline::default(),
                                    );
                                }
                                if now.elapsed() > Duration::from_secs(3) {
                                    entry.2 = Deadline::default();
                                }
                                let due = entry.2.due(
                                    Instant::now(),
                                    eligible,
                                    Duration::from_secs(ANTI_AFK_SECONDS),
                                );
                                if due {
                                    if activity_busy {
                                        entry.2.reschedule(Instant::now(), Duration::from_secs(60));
                                    } else if account.client.mode
                                        == crate::client::ClientMode::Headless
                                    {
                                        if self.activity_eligible(&account) {
                                            activity_busy = control
                                                .start_activity(&process, account.anti_afk.action);
                                        }
                                        entry.2.reschedule(Instant::now(), Duration::from_secs(60));
                                    } else {
                                        control.input(account.anti_afk.action, || {
                                            !self.activity_eligible(&account)
                                        });
                                    }
                                }
                                control.observed_mode(&mut result);
                                control.activity_status(&mut result);
                                if account.client.monitoring {
                                    gpu.apply(identity, &mut result);
                                }
                                if result.gpu_percent.is_none() {
                                    result.unavailable.push(
                                        "GPU engine measurement unavailable for verified process"
                                            .into(),
                                    );
                                }
                                if result.dedicated_gpu_memory_bytes.is_none() {
                                    result.unavailable.push("Dedicated GPU memory unavailable or requires individual adapter counters".into());
                                }
                            }
                            Err(error) => {
                                let _ = control
                                    .discard_exited_journal(self.data_directory(), &account.id);
                                result.resource_result = error;
                                result.input_result =
                                    "Input unavailable: ownership verification failed".into();
                                result
                                    .unavailable
                                    .push("Process ownership could not be verified".into());
                            }
                        }
                    }
                    #[cfg(not(windows))]
                    {
                        result.resource_result =
                            "Windows controls unavailable on this platform".into();
                        result.input_result = "Anti-AFK unavailable on this platform".into();
                        result.unavailable.push(
                            "CPU, working set and GPU monitoring unavailable on this platform"
                                .into(),
                        );
                    }
                    #[cfg(windows)]
                    if let Some(error) = environment.error() {
                        result.unavailable.push(error.into());
                    }
                    runtime.insert(account.id.clone(), result);
                }
                let mut state = self.0.state.lock().unwrap();
                runtime.retain(|id, result| {
                    state.database.accounts.iter().any(|a| {
                        a.account.id == *id
                            && a.account.process == result.identity
                            && result
                                .identity
                                .as_ref()
                                .is_some_and(|p| p.generation == a.account.generation)
                    })
                });
                state.runtime_controls = runtime;
                let mut event = snapshot(&state);
                event.session_suspended = suspended;
                let _ = self.0.events.send(event);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        #[cfg(windows)]
        {
            let mut cleanup_errors = Vec::new();
            for control in managed.values_mut().chain(retiring.iter_mut()) {
                let until = Instant::now() + Duration::from_secs(10);
                loop {
                    match control.restore() {
                        Ok(()) => break,
                        Err(error) if Instant::now() >= until => {
                            cleanup_errors.push(error);
                            break;
                        }
                        Err(_) => {
                            environment.pump();
                            std::thread::sleep(Duration::from_millis(20));
                        }
                    }
                }
            }
            let report = serde_json::json!({"completed_at": Utc::now(), "errors": cleanup_errors});
            let _ = std::fs::write(
                self.data_directory().join("control-cleanup.json"),
                report.to_string(),
            );
        }
    }
}

#[cfg(any(windows, test))]
fn activity_allowed(account: &Account) -> bool {
    account.desired_running
        && account.status == Status::Running
        && account.anti_afk.enabled
        && match account.client.mode {
            crate::client::ClientMode::Normal => !account.client.auto_hide,
            crate::client::ClientMode::Headless => account.anti_afk.headless_foreground_activity,
            crate::client::ClientMode::NetworkOnly => false,
        }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn headless_requires_separate_opt_in_and_running_session() {
        let mut a = Account::new("1".into(), "fixture".into());
        a.desired_running = true;
        a.status = Status::Running;
        a.anti_afk.enabled = true;
        assert!(activity_allowed(&a));
        a.client.mode = crate::client::ClientMode::Headless;
        assert!(!activity_allowed(&a));
        a.anti_afk.headless_foreground_activity = true;
        assert!(activity_allowed(&a));
        a.status = Status::Reconnecting;
        assert!(!activity_allowed(&a));
        a.status = Status::Running;
        a.client.mode = crate::client::ClientMode::NetworkOnly;
        assert!(!activity_allowed(&a));
        a.client.mode = crate::client::ClientMode::Normal;
        a.client.auto_hide = true;
        assert!(!activity_allowed(&a));
    }
}
