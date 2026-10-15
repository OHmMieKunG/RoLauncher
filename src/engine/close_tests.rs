use super::*;

struct Fixture {
    engine: Engine,
    path: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("pr07-effects-{}", Uuid::new_v4()));
        let engine = Engine::open(Store::new(path.clone()).unwrap(), "synthetic".into()).unwrap();
        engine
            .transaction(|s| {
                for id in ["1", "2"] {
                    let mut a = Account::new(id.into(), format!("test{id}"));
                    a.target = Some(Target {
                        place_id: 1,
                        job_id: None,
                        private_server_link: None,
                    });
                    a.desired_running = true;
                    a.status = Status::Running;
                    a.operation_id = Some(Uuid::new_v4());
                    a.tracker = Some(format!("12345678901{id}"));
                    a.process = Some(ProcessIdentity {
                        pid: 4200 + id.parse::<u32>().unwrap(),
                        creation_time: format!("synthetic-{id}"),
                        tracker: a.tracker.clone().unwrap(),
                        generation: a.generation,
                    });
                    s.database.accounts.push(SavedAccount {
                        account: a,
                        encrypted_session: String::new(),
                    });
                }
                Ok(())
            })
            .unwrap();
        Self { engine, path }
    }
    fn account(&self) -> Account {
        self.engine.snapshot().accounts.remove(0)
    }
    fn players(&self) -> Vec<platform::Player> {
        self.engine
            .snapshot()
            .accounts
            .into_iter()
            .filter_map(|a| a.process)
            .map(|p| platform::Player {
                pid: p.pid,
                creation_time: p.creation_time,
                tracker: p.tracker,
            })
            .collect()
    }
    fn tick(&self, expected: &Account, now: Instant, attempts: &mut CloseAttempts) -> Vec<bool> {
        let mut effects = Vec::new();
        let players = self.players();
        let _ = self.engine.close_owned(
            expected,
            now,
            attempts,
            || Ok(players),
            |_, force| {
                effects.push(force);
                Ok(())
            },
        );
        effects
    }
    fn recovery(&self) {
        let a = self.account();
        self.engine.signal("1", a.generation, Signal::Disconnected);
        self.engine
            .update("1", a.generation, |a| {
                a.disconnected_since = Some(Utc::now() - ChronoDuration::seconds(31))
            })
            .unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.path).unwrap();
    }
}

#[test]
fn restart_closes_current_generation_but_stale_restart_cannot_close_newer_client() {
    let f = Fixture::new();
    let old = f.account();
    f.engine.command("1", "restart").unwrap();
    let mut attempts = CloseAttempts::default();
    assert!(f.tick(&old, Instant::now(), &mut attempts).is_empty());
    assert_eq!(
        f.tick(&f.account(), Instant::now(), &mut attempts),
        vec![false]
    );
    let current = f.account();
    f.engine.command("1", "stop").unwrap();
    assert!(f.tick(&current, Instant::now(), &mut attempts).is_empty());
    assert_eq!(
        f.tick(&f.account(), Instant::now(), &mut attempts),
        vec![false]
    );
}
#[test]
fn recovery_and_force_escalation_remain_available_under_the_same_valid_intent() {
    let f = Fixture::new();
    f.recovery();
    let now = Instant::now();
    let mut attempts = CloseAttempts::default();
    assert_eq!(f.tick(&f.account(), now, &mut attempts), vec![false]);
    assert_eq!(f.account().failures, 1);
    assert_eq!(
        f.engine.activity(Some("1")).first().unwrap().status,
        Status::Backoff
    );
    assert_eq!(
        f.tick(&f.account(), now + Duration::from_secs(6), &mut attempts),
        vec![true]
    );
    assert_eq!(f.account().failures, 1);
}
#[test]
fn recovery_disabled_after_graceful_close_cannot_force_or_consume_more_retries() {
    let f = Fixture::new();
    f.recovery();
    let now = Instant::now();
    let mut attempts = CloseAttempts::default();
    assert_eq!(f.tick(&f.account(), now, &mut attempts), vec![false]);
    f.engine
        .patch(
            "1",
            AccountPatch {
                auto_recovery: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
    for offset in [6, 12, 20] {
        assert!(
            f.tick(
                &f.account(),
                now + Duration::from_secs(offset),
                &mut attempts
            )
            .is_empty()
        );
    }
    assert_eq!(f.account().failures, 1);
    assert!(f.account().process.is_some());
    f.engine
        .patch(
            "1",
            AccountPatch {
                auto_recovery: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(
        f.tick(&f.account(), now + Duration::from_secs(30), &mut attempts)
            .is_empty()
    );
    f.engine.command("1", "stop").unwrap();
    f.engine.set_launch_allowed(false);
    assert_eq!(
        f.tick(&f.account(), now + Duration::from_secs(40), &mut attempts),
        vec![false]
    );
}
#[test]
fn revoked_replacement_before_first_effect_preserves_process_and_retry_budget() {
    let f = Fixture::new();
    f.recovery();
    let a = f.account();
    f.engine
        .patch(
            "1",
            AccountPatch {
                auto_recovery: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(
        f.tick(&a, Instant::now(), &mut CloseAttempts::default())
            .is_empty()
    );
    assert!(
        f.tick(&f.account(), Instant::now(), &mut CloseAttempts::default())
            .is_empty()
    );
    assert_eq!(f.account().failures, 0);
    assert!(f.account().process.is_some());
}
#[test]
fn ambiguous_tracker_pid_reuse_and_shared_account_ownership_all_deny_close() {
    for scenario in ["ambiguous", "pid-reuse", "creation-time", "shared-owner"] {
        let f = Fixture::new();
        f.engine.command("1", "stop").unwrap();
        let a = f.account();
        let mut players = f.players();
        match scenario {
            "ambiguous" => players.push(platform::Player {
                pid: 9999,
                creation_time: "other".into(),
                tracker: a.tracker.clone().unwrap(),
            }),
            "pid-reuse" => players[0].pid = 9999,
            "creation-time" => players[0].creation_time = "reused".into(),
            _ => {
                f.engine
                    .transaction(|s| {
                        let p = s.database.accounts[0].account.process.clone();
                        s.database.accounts[1].account.process = p;
                        Ok(())
                    })
                    .unwrap();
            }
        }
        let mut closed = false;
        let _ = f.engine.close_owned(
            &a,
            Instant::now(),
            &mut CloseAttempts::default(),
            || Ok(players),
            |_, _| {
                closed = true;
                Ok(())
            },
        );
        assert!(!closed, "{scenario}");
        assert!(f.account().process.is_some());
    }
}
#[test]
fn failed_graceful_close_never_becomes_a_force_attempt_on_later_ticks() {
    let f = Fixture::new();
    f.engine.command("1", "stop").unwrap();
    let now = Instant::now();
    let mut attempts = CloseAttempts::default();
    let players = f.players();
    let _ = f.engine.close_owned(
        &f.account(),
        now,
        &mut attempts,
        || Ok(players),
        |_, force| {
            assert!(!force);
            Err("simulated failure".into())
        },
    );
    for offset in [6, 20] {
        assert!(
            f.tick(
                &f.account(),
                now + Duration::from_secs(offset),
                &mut attempts
            )
            .is_empty()
        );
    }
    assert_eq!(f.account().failures, 0);
}
#[test]
fn connected_signal_revokes_recovery_before_force_escalation() {
    let f = Fixture::new();
    f.recovery();
    let now = Instant::now();
    let mut attempts = CloseAttempts::default();
    assert_eq!(f.tick(&f.account(), now, &mut attempts), vec![false]);
    let a = f.account();
    f.engine.signal("1", a.generation, Signal::Connected);
    assert!(
        f.tick(&f.account(), now + Duration::from_secs(6), &mut attempts)
            .is_empty()
    );
    assert_eq!(f.account().status, Status::Running);
}
#[test]
fn target_replacement_requires_current_fallback_and_recovery_settings() {
    for enabled in [true, false] {
        let f = Fixture::new();
        let a = f.account();
        f.engine
            .patch(
                "1",
                AccountPatch {
                    auto_recovery: Some(enabled),
                    fallback_policy: Some(FallbackPolicy::AllowPublic),
                    ..Default::default()
                },
            )
            .unwrap();
        f.engine
            .signal("1", a.generation, Signal::TargetUnavailable);
        let effects = f.tick(&f.account(), Instant::now(), &mut CloseAttempts::default());
        assert_eq!(effects, if enabled { vec![false] } else { vec![] });
    }
}
#[tokio::test]
async fn stop_during_awaited_recovery_supersedes_the_old_effect() {
    let f = Fixture::new();
    f.recovery();
    let old = f.account();
    let (send, recv) = tokio::sync::oneshot::channel();
    let engine = f.engine.clone();
    let players = f.players();
    let pending = tokio::spawn(async move {
        recv.await.unwrap();
        let mut effects = Vec::new();
        let _ = engine.close_owned(
            &old,
            Instant::now(),
            &mut CloseAttempts::default(),
            || Ok(players),
            |_, force| {
                effects.push(force);
                Ok(())
            },
        );
        effects
    });
    f.engine.command("1", "stop").unwrap();
    send.send(()).unwrap();
    assert!(pending.await.unwrap().is_empty());
    f.engine.set_launch_allowed(false);
    assert_eq!(
        f.tick(&f.account(), Instant::now(), &mut CloseAttempts::default()),
        vec![false]
    );
}
#[test]
fn legacy_live_state_is_observation_only_until_explicit_stop() {
    let f = Fixture::new();
    let a = f.account();
    f.engine
        .update("1", a.generation, |a| {
            a.desired_running = false;
            a.status = Status::Stopped;
        })
        .unwrap();
    assert!(
        f.tick(&f.account(), Instant::now(), &mut CloseAttempts::default())
            .is_empty()
    );
    f.engine.command("1", "stop").unwrap();
    assert_eq!(
        f.tick(&f.account(), Instant::now(), &mut CloseAttempts::default()),
        vec![false]
    );
}
#[test]
fn coordination_loss_during_process_discovery_prevents_replacement_effect() {
    let f = Fixture::new();
    f.engine.command("1", "restart").unwrap();
    let players = f.players();
    let mut closed = false;
    let gate = f.engine.0.launch_allowed.clone();
    let _ = f.engine.close_owned(
        &f.account(),
        Instant::now(),
        &mut CloseAttempts::default(),
        || {
            gate.store(false, Ordering::Release);
            Ok(players)
        },
        |_, _| {
            closed = true;
            Ok(())
        },
    );
    assert!(!closed);
}
#[test]
fn denied_close_stays_paused_after_reopen_even_if_client_exits_naturally() {
    let f = Fixture::new();
    f.engine.command("1", "restart").unwrap();
    f.engine.set_launch_allowed(false);
    assert!(
        f.tick(&f.account(), Instant::now(), &mut CloseAttempts::default())
            .is_empty()
    );
    let a = f.account();
    f.engine
        .update("1", a.generation, |a| {
            a.process = None;
            a.tracker = None;
        })
        .unwrap();
    let reopened = Engine::open(Store::new(f.path.clone()).unwrap(), "test".into()).unwrap();
    assert_eq!(
        reopened.snapshot().accounts[0].status,
        Status::NeedsAttention
    );
}
#[test]
fn retry_now_preserves_an_eligible_target_replacement_intent() {
    let f = Fixture::new();
    let a = f.account();
    f.engine
        .patch(
            "1",
            AccountPatch {
                fallback_policy: Some(FallbackPolicy::AllowPublic),
                ..Default::default()
            },
        )
        .unwrap();
    f.engine
        .signal("1", a.generation, Signal::TargetUnavailable);
    f.engine.retry_now("1").unwrap();
    assert_eq!(
        f.tick(&f.account(), Instant::now(), &mut CloseAttempts::default()),
        vec![false]
    );
}

#[test]
fn close_failure_remains_latched_when_the_denial_cannot_be_persisted() {
    let f = Fixture::new();
    f.engine.command("1", "stop").unwrap();
    let players = f.players();
    let now = Instant::now();
    let mut attempts = CloseAttempts::default();
    let _ = f.engine.close_owned(
        &f.account(),
        now,
        &mut attempts,
        || Ok(players),
        |_, _| {
            std::fs::create_dir(f.path.join("accounts.json.tmp")).unwrap();
            Err("simulated close and persistence failure".into())
        },
    );
    assert!(
        f.tick(&f.account(), now + Duration::from_secs(6), &mut attempts)
            .is_empty()
    );
}
#[test]
fn another_retired_record_cannot_claim_the_same_pid_under_a_different_tracker() {
    let f = Fixture::new();
    f.engine.command("1", "stop").unwrap();
    let a = f.account();
    f.engine
        .transaction(|s| {
            s.database.retired_launches.push(RetiredLaunch {
                tracker: "987654321012".into(),
                generation: Uuid::new_v4(),
                account_id: "2".into(),
                deadline: Utc::now() + ChronoDuration::seconds(90),
                close_intent: None,
                process: a.process.clone(),
            });
            Ok(())
        })
        .unwrap();
    assert!(
        f.tick(&a, Instant::now(), &mut CloseAttempts::default())
            .is_empty()
    );
}
#[tokio::test]
async fn obsolete_pending_launch_blocks_replacement_before_credentials_or_dispatch() {
    let f = Fixture::new();
    let a = f.account();
    f.engine
        .update("1", a.generation, |a| {
            a.process = None;
            a.status = Status::NeedsAttention;
        })
        .unwrap();
    f.engine.command("1", "start").unwrap();
    let queued = f.account();
    f.engine.launch_one(queued).await;
    assert_eq!(f.account().status, Status::Queued);
    assert!(f.account().last_error.is_none());
    assert_eq!(f.account().failures, 0);
}
#[test]
fn persistence_failure_prevents_the_destructive_effect() {
    let f = Fixture::new();
    f.recovery();
    std::fs::create_dir(f.path.join("accounts.json.tmp")).unwrap();
    assert!(
        f.tick(&f.account(), Instant::now(), &mut CloseAttempts::default())
            .is_empty()
    );
    assert_eq!(f.account().failures, 0);
}

fn pending_stop(f: &Fixture) -> (RetiredLaunch, Vec<platform::Player>) {
    let a = f.account();
    let identity = a.process.clone().unwrap();
    f.engine
        .update("1", a.generation, |a| a.process = None)
        .unwrap();
    f.engine.command("1", "stop").unwrap();
    let old = Store::new(f.path.clone())
        .unwrap()
        .load()
        .unwrap()
        .retired_launches
        .remove(0);
    (
        old,
        vec![platform::Player {
            pid: identity.pid,
            creation_time: identity.creation_time,
            tracker: identity.tracker,
        }],
    )
}
#[test]
fn late_explicit_stop_closes_and_force_requires_the_same_current_stop() {
    let f = Fixture::new();
    let (old, players) = pending_stop(&f);
    let now = Instant::now();
    let mut attempts = CloseAttempts::default();
    let mut effects = Vec::new();
    f.engine
        .close_retired(
            &old,
            now,
            &mut attempts,
            || Ok(players.clone()),
            |_, force| {
                effects.push(force);
                Ok(())
            },
        )
        .unwrap();
    let current = Store::new(f.path.clone())
        .unwrap()
        .load()
        .unwrap()
        .retired_launches
        .remove(0);
    f.engine
        .close_retired(
            &current,
            now + Duration::from_secs(6),
            &mut attempts,
            || Ok(players.clone()),
            |_, force| {
                effects.push(force);
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(effects, vec![false, true]);
    let a = f.account();
    f.engine
        .update("1", a.generation, |a| a.desired_running = true)
        .unwrap();
    f.engine
        .close_retired(
            &current,
            now + Duration::from_secs(12),
            &mut attempts,
            || Ok(players),
            |_, _| panic!("revoked Stop cannot force"),
        )
        .unwrap();
    assert_eq!(f.account().status, Status::NeedsAttention);
}
#[test]
fn retired_ambiguous_obsolete_expired_and_reused_identities_never_close() {
    for scenario in ["ambiguous", "obsolete", "expired", "pid-reuse"] {
        let f = Fixture::new();
        let (mut old, mut players) = pending_stop(&f);
        let mut attempts = CloseAttempts::default();
        if scenario == "pid-reuse" {
            f.engine
                .close_retired(
                    &old,
                    Instant::now(),
                    &mut attempts,
                    || Ok(players.clone()),
                    |_, _| Ok(()),
                )
                .unwrap();
            old = Store::new(f.path.clone())
                .unwrap()
                .load()
                .unwrap()
                .retired_launches
                .remove(0);
            players[0].creation_time = "reused".into();
        } else if scenario == "ambiguous" {
            players.push(platform::Player {
                pid: 9999,
                creation_time: "other".into(),
                tracker: old.tracker.clone(),
            });
        } else {
            f.engine
                .transaction(|s| {
                    let old = &mut s.database.retired_launches[0];
                    if scenario == "obsolete" {
                        old.close_intent.as_mut().unwrap().reason = CloseReason::ObsoleteLaunch;
                    } else {
                        old.deadline = Utc::now() - ChronoDuration::seconds(1);
                    }
                    Ok(())
                })
                .unwrap();
            old = Store::new(f.path.clone())
                .unwrap()
                .load()
                .unwrap()
                .retired_launches
                .remove(0);
        }
        let mut closed = false;
        f.engine
            .close_retired(
                &old,
                Instant::now() + Duration::from_secs(10),
                &mut attempts,
                || Ok(players),
                |_, _| {
                    closed = true;
                    Ok(())
                },
            )
            .unwrap();
        assert!(!closed, "{scenario}");
        assert_eq!(f.account().status, Status::NeedsAttention);
    }
}
#[test]
fn retired_binding_survives_reopen_and_blocks_replacement_after_discovery_window() {
    let f = Fixture::new();
    let (old, players) = pending_stop(&f);
    f.engine
        .close_retired(
            &old,
            Instant::now(),
            &mut CloseAttempts::default(),
            || Ok(players),
            |_, _| Ok(()),
        )
        .unwrap();
    f.engine
        .transaction(|s| {
            s.database.retired_launches[0].deadline = Utc::now() - ChronoDuration::seconds(1);
            Ok(())
        })
        .unwrap();
    let reopened = Engine::open(Store::new(f.path.clone()).unwrap(), "test".into()).unwrap();
    assert!(reopened.command("1", "start").is_err());
    assert!(reopened.remove("1").is_err());
    reopened
        .transaction(|s| {
            let a = account_mut(s, "2")?;
            a.desired_running = false;
            a.process = None;
            a.tracker = None;
            for saved in &mut s.database.accounts {
                saved.encrypted_session = platform::protect("synthetic-credential")?;
            }
            Ok(())
        })
        .unwrap();
    let backup = reopened.backup().unwrap();
    let restored = Store::new(f.path.clone())
        .unwrap()
        .read_backup(&backup)
        .unwrap();
    assert!(
        restored
            .accounts
            .iter()
            .all(|a| a.account.close_intent.is_none())
    );
    assert!(
        reopened.restore(&backup).is_err(),
        "restore must retain expired but still-owned retired evidence"
    );
    let db = Store::new(f.path.clone()).unwrap().load().unwrap();
    assert_eq!(db.retired_launches[0].process.as_ref().unwrap().pid, 4201);
}
#[test]
fn concurrent_commands_and_close_ticks_never_target_another_accounts_client() {
    let f = Fixture::new();
    let other = f.engine.snapshot().accounts[1].process.clone();
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let mut threads = Vec::new();
    for actions in [["restart", "stop", "start"], ["stop", "start", "restart"]] {
        let engine = f.engine.clone();
        let barrier = barrier.clone();
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            for _ in 0..4 {
                for action in actions {
                    let _ = engine.command("1", action);
                }
            }
        }));
    }
    barrier.wait();
    let mut attempts = CloseAttempts::default();
    for _ in 0..12 {
        let a = f.account();
        let players = f.players();
        let _ = f.engine.close_owned(
            &a,
            Instant::now(),
            &mut attempts,
            || Ok(players),
            |p, _| {
                assert_eq!(p.pid, 4201);
                assert_eq!(p.creation_time, "synthetic-1");
                Ok(())
            },
        );
    }
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(f.engine.snapshot().accounts[1].process, other);
}
#[cfg(windows)]
#[test]
fn native_windows_close_rejects_a_reused_or_unrelated_process_identity() {
    let identity = ProcessIdentity {
        pid: std::process::id(),
        creation_time: "synthetic-wrong-creation-time".into(),
        tracker: "123456789012".into(),
        generation: Uuid::new_v4(),
    };
    assert!(platform::close(&identity, false).is_err());
    assert!(platform::close(&identity, true).is_err());
}

#[test]
fn legacy_retired_record_observes_without_closing() {
    let path = std::env::temp_dir().join(format!("pr07-legacy-{}", Uuid::new_v4()));
    let engine = Engine::open(Store::new(path.clone()).unwrap(), "test".into()).unwrap();
    let old: RetiredLaunch = serde_json::from_value(serde_json::json!({
        "tracker":"123456789012", "generation":Uuid::new_v4(), "account_id":"1", "deadline":Utc::now()+ChronoDuration::seconds(90)
    })).unwrap();
    engine
        .transaction(|s| {
            s.database.accounts.push(SavedAccount {
                account: Account::new("1".into(), "test".into()),
                encrypted_session: String::new(),
            });
            s.database.retired_launches.push(old.clone());
            Ok(())
        })
        .unwrap();
    let mut closed = false;
    engine
        .close_retired(
            &old,
            Instant::now(),
            &mut CloseAttempts::default(),
            || {
                Ok(vec![platform::Player {
                    pid: 4242,
                    creation_time: "synthetic-1".into(),
                    tracker: old.tracker.clone(),
                }])
            },
            |_, _| {
                closed = true;
                Ok(())
            },
        )
        .unwrap();
    assert!(!closed);
    assert_eq!(engine.snapshot().accounts[0].status, Status::NeedsAttention);
    let db = Store::new(path.clone()).unwrap().load().unwrap();
    assert_eq!(db.retired_launches[0].process.as_ref().unwrap().pid, 4242);
    assert!(db.retired_launches[0].close_intent.is_none());
    engine.command("1", "stop").unwrap();
    let db = Store::new(path.clone()).unwrap().load().unwrap();
    engine
        .close_retired(
            &db.retired_launches[0],
            Instant::now(),
            &mut CloseAttempts::default(),
            || {
                Ok(vec![platform::Player {
                    pid: 4242,
                    creation_time: "synthetic-1".into(),
                    tracker: old.tracker.clone(),
                }])
            },
            |_, _| {
                closed = true;
                Ok(())
            },
        )
        .unwrap();
    assert!(
        closed,
        "explicit Stop must recover a uniquely observed legacy client"
    );
    drop(engine);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn explicit_stop_closes_only_the_verified_owned_process() {
    let path = std::env::temp_dir().join(format!("pr07-{}", Uuid::new_v4()));
    let engine = Engine::open(Store::new(path.clone()).unwrap(), "test".into()).unwrap();
    engine
        .transaction(|s| {
            let mut a = Account::new("1".into(), "test".into());
            a.desired_running = true;
            a.process = Some(ProcessIdentity {
                pid: 4242,
                creation_time: "synthetic-1".into(),
                tracker: "123456789012".into(),
                generation: a.generation,
            });
            a.tracker = Some("123456789012".into());
            s.database.accounts.push(SavedAccount {
                account: a,
                encrypted_session: String::new(),
            });
            Ok(())
        })
        .unwrap();
    engine.command("1", "stop").unwrap();
    let a = engine.snapshot().accounts.remove(0);
    let identity = a.process.clone().unwrap();
    let player = platform::Player {
        pid: 4242,
        creation_time: "synthetic-1".into(),
        tracker: "123456789012".into(),
    };
    let mut closed = Vec::new();
    engine
        .close_owned(
            &a,
            Instant::now(),
            &mut CloseAttempts::default(),
            || Ok(vec![player]),
            |p, force| {
                closed.push((p.clone(), force));
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(closed, vec![(identity, false)]);
    drop(engine);
    std::fs::remove_dir_all(path).unwrap();
}
