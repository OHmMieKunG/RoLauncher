//! Explicit, ignored live tests. No acceptance code is included in the shipped launcher.
use super::*;
use crate::{bootstrapper::BootstrapperProvider, client::Bootstrapper, model::Account};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs::File, io::Write, path::PathBuf, time::Duration};

type Result<T> = std::result::Result<T, &'static str>;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Scenario {
    Switches,
    Benchmark,
    Idle25,
    Idle60,
    Activity25,
    Activity60,
    Recovery,
    ServerDisconnect,
    Isolation,
    Occlusion,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    base_url: String,
    account_ids: [String; 2],
    place_id: u64,
    scenario: Scenario,
    output: PathBuf,
    presentmon: Option<PathBuf>,
    authorized_test_experience: bool,
}
impl Config {
    fn validate(&self) -> Result<()> {
        let url = url::Url::parse(&self.base_url).map_err(|_| "Invalid API endpoint")?;
        if url.scheme() != "http"
            || url.host_str() != Some("127.0.0.1")
            || url.port().is_none()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err("Use http://127.0.0.1:PORT without credentials, path or query");
        }
        if !self.authorized_test_experience || self.place_id == 0 {
            return Err("An explicitly authorized place is required");
        }
        if self.account_ids[0] == self.account_ids[1]
            || self
                .account_ids
                .iter()
                .any(|id| id.is_empty() || id.len() > 20 || !id.bytes().all(|c| c.is_ascii_digit()))
        {
            return Err("Exactly two distinct numeric account IDs are required");
        }
        if !self.output.is_absolute()
            || self
                .output
                .file_name()
                .and_then(|n| n.to_str())
                .is_none_or(|n| {
                    n.is_empty()
                        || n.len() > 64
                        || !n
                            .bytes()
                            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
                })
            || self
                .presentmon
                .as_ref()
                .is_some_and(|p| !p.is_absolute() || !p.is_file())
        {
            return Err("Use absolute output and existing capture-tool paths");
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct Snapshot {
    accounts: Vec<Account>,
    runtime_controls: BTreeMap<String, Value>,
    session_suspended: bool,
    network_suspended: bool,
}

struct Api {
    client: reqwest::Client,
    base: String,
    token: zeroize::Zeroizing<String>,
}
impl Api {
    async fn request(&self, method: reqwest::Method, path: &str, body: Value) -> Result<Value> {
        let response = self
            .client
            .request(method, format!("{}{path}", self.base.trim_end_matches('/')))
            .bearer_auth(self.token.as_str())
            .json(&body)
            .send()
            .await
            .map_err(|_| "Local API request failed; details withheld")?;
        if !response.status().is_success() {
            return Err("Local API rejected request; inspect sanitized launcher UI");
        }
        let bytes = response.bytes().await.map_err(|_| "API response failed")?;
        if bytes.is_empty() {
            Ok(Value::Null)
        } else {
            serde_json::from_slice(&bytes).map_err(|_| "Invalid API JSON")
        }
    }
    async fn snapshot(&self) -> Result<Snapshot> {
        serde_json::from_value(
            self.request(reqwest::Method::GET, "/v1/status", Value::Null)
                .await?,
        )
        .map_err(|_| "Invalid status schema")
    }
    async fn patch(&self, account: &Account) -> Result<()> {
        self.request(
            reqwest::Method::PATCH,
            &format!("/v1/accounts/{}", account.id),
            json!({"client": account.client, "resource": account.resource,
                "anti_afk": account.anti_afk, "auto_recovery": account.auto_recovery}),
        )
        .await?;
        Ok(())
    }
    async fn action(&self, id: &str, action: &str) -> Result<()> {
        self.request(
            reqwest::Method::POST,
            &format!("/v1/accounts/{id}/{action}"),
            Value::Null,
        )
        .await?;
        Ok(())
    }
}

fn selected<'a>(snapshot: &'a Snapshot, ids: &[String; 2]) -> Result<Vec<&'a Account>> {
    if snapshot.accounts.len() != 2 {
        return Err("Use an isolated launcher containing only the two dedicated test accounts");
    }
    ids.iter()
        .map(|id| {
            snapshot
                .accounts
                .iter()
                .find(|a| &a.id == id)
                .ok_or("Selected account is missing")
        })
        .collect()
}

fn unchanged(expected: &Account, actual: &Account) -> Result<()> {
    if actual.generation != expected.generation
        || actual.process != expected.process
        || actual.connected_since != expected.connected_since
        || actual.last_disconnect != expected.last_disconnect
        || actual.reconnection_attempts != expected.reconnection_attempts
        || actual.status != crate::model::Status::Running
    {
        return Err("Process or connection continuity changed");
    }
    if actual.client != expected.client
        || actual.resource != expected.resource
        || actual.target != expected.target
        || actual.anti_afk != expected.anti_afk
        || actual.auto_recovery != expected.auto_recovery
    {
        return Err("Account preferences changed unexpectedly");
    }
    Ok(())
}

struct Journal(File);
impl Journal {
    fn event(&mut self, mut record: Value) -> Result<()> {
        record["utc"] = json!(chrono::Utc::now());
        serde_json::to_writer(&mut self.0, &record).map_err(|_| "Cannot write evidence")?;
        self.0
            .write_all(b"\n")
            .map_err(|_| "Cannot write evidence")?;
        self.0.flush().map_err(|_| "Cannot flush evidence")
    }
}

fn public_account(a: &Account, runtime: Option<&Value>) -> Value {
    json!({"account_id": a.id, "provider": a.client.bootstrapper,
        "requested_mode": a.client.mode, "resource": a.resource.mode,
        "status": a.status, "pid": a.process.as_ref().map(|p| p.pid),
        "creation_time": a.process.as_ref().map(|p| &p.creation_time),
        "generation": a.generation, "connected_since": a.connected_since,
        "last_disconnect": a.last_disconnect, "reconnection_attempts": a.reconnection_attempts,
        "observed_mode": runtime.and_then(|r| r.get("client_mode")),
        "observed_window": runtime.and_then(|r| r.get("window_state")),
        "last_activity_attempt_at": runtime.and_then(|r| r.get("last_activity_attempt_at")),
        "last_input_submitted_at": runtime.and_then(|r| r.get("last_input_submitted_at"))})
}

fn native_sample(managed: &mut Managed, account: &Account, hwnd: HWND) -> Result<Value> {
    let identity = account.process.as_ref().ok_or("No managed process")?;
    let process = platform::verify_controls(identity, account.generation)
        .map_err(|_| "Full process ownership verification failed")?;
    if !hwnd.is_null() && !window_owned(hwnd, identity.pid) {
        return Err("Owned window was destroyed or replaced; repeat this measurement block");
    }
    let mut runtime = RuntimeControls::default();
    managed.sample(&process, &mut runtime);
    let mut memory: PROCESS_MEMORY_COUNTERS_EX = unsafe { std::mem::zeroed() };
    memory.cb = size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
    let private = unsafe {
        (GetProcessMemoryInfo(
            process.raw(),
            (&mut memory as *mut PROCESS_MEMORY_COUNTERS_EX).cast(),
            memory.cb,
        ) != 0)
            .then_some(memory.PrivateUsage as u64)
    };
    Ok(json!({"cpu_percent_machine": runtime.cpu_percent,
        "working_set_bytes": runtime.working_set_bytes, "private_bytes": private,
        "page_faults_per_second": runtime.page_faults_per_second,
        "process_uptime_seconds": runtime.process_uptime_seconds,
        "priority": unsafe { GetPriorityClass(process.raw()) },
        "power_control_mask": power_policy(&process).ok().map(|p| p.ControlMask),
        "power_state_mask": power_policy(&process).ok().map(|p| p.StateMask),
        "memory_priority": memory_priority(&process).ok(),
        "page_fault_count": private.map(|_| memory.PageFaultCount),
        "window_iconic": !hwnd.is_null() && unsafe { IsIconic(hwnd) } != 0,
        "effective_profile": account.resource.effective_mode(account.client.mode),
        "window": if hwnd.is_null() { "unavailable" } else if window_matches(hwnd, SW_HIDE) { "hidden" }
            else if window_matches(hwnd, SW_MINIMIZE) { "minimized" } else { "visible" },
        "foreground": unsafe { GetForegroundWindow() == hwnd }}))
}

fn gpu_samples(pids: &[u32]) -> Value {
    #[derive(Deserialize, serde::Serialize)]
    #[serde(rename_all = "PascalCase")]
    struct Row {
        name: String,
        utilization_percentage: u64,
    }
    // Preserve the adapter LUID, physical index and engine name; never sum engines.
    let rows = wmi::WMIConnection::new().ok().and_then(|connection| {
        connection.raw_query::<Row>("SELECT Name, UtilizationPercentage FROM Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine").ok()
    });
    rows.map_or(Value::Null, |rows| {
        json!(
            rows.into_iter()
                .filter(|r| {
                    r.name
                        .strip_prefix("pid_")
                        .and_then(|s| s.split('_').next())
                        .and_then(|s| s.parse::<u32>().ok())
                        .is_some_and(|pid| pids.contains(&pid))
                })
                .collect::<Vec<_>>()
        )
    })
}

struct Capture {
    child: std::process::Child,
    tool: PathBuf,
    session: String,
}
impl Drop for Capture {
    fn drop(&mut self) {
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            use std::os::windows::process::CommandExt;
            // Stop only this collector's UUID-named ETW session, never a pre-existing capture.
            if let Ok(mut stop) = std::process::Command::new(&self.tool)
                .args([
                    "--session_name",
                    &self.session,
                    "--terminate_existing_session",
                ])
                .creation_flags(0x08000000)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
            {
                let until = Instant::now() + Duration::from_secs(2);
                while matches!(stop.try_wait(), Ok(None)) && Instant::now() < until {
                    std::thread::sleep(Duration::from_millis(20));
                }
                let _ = stop.kill();
                let _ = stop.wait();
            }
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}
fn capture(config: &Config, pid: u32, label: &str) -> Result<Option<Capture>> {
    let Some(tool) = &config.presentmon else {
        return Ok(None);
    };
    use std::os::windows::process::CommandExt;
    let session = format!("RoLauncher-{}", uuid::Uuid::new_v4());
    let stderr = File::options()
        .write(true)
        .create_new(true)
        .open(config.output.join(format!("{label}-presentmon-stderr.txt")))
        .map_err(|_| "Cannot create capture diagnostic file")?;
    let stdout = File::options()
        .write(true)
        .create_new(true)
        .open(config.output.join(format!("{label}-presentmon-stdout.txt")))
        .map_err(|_| "Cannot create capture diagnostic file")?;
    std::process::Command::new(tool)
        .args([
            "--process_id",
            &pid.to_string(),
            "--timed",
            "60",
            "--terminate_after_timed",
            "--qpc_time_ms",
            "--no_track_input",
            "--v2_metrics",
            "--track_hybrid_present",
            "--no_console_stats",
            "--session_name",
            &session,
            "--output_file",
        ])
        .arg(config.output.join(format!("{label}-presentmon.csv")))
        .creation_flags(0x08000000)
        .stdin(std::process::Stdio::null())
        .stdout(stdout)
        .stderr(stderr)
        .spawn()
        .map(|child| Capture {
            child,
            tool: tool.clone(),
            session,
        })
        .map(Some)
        .map_err(|_| "PresentMon could not start")
}

// Only compiled into this explicit test harness, never the launcher.
struct PowerExperiment {
    process: platform::ControlProcess,
    original: PROCESS_POWER_THROTTLING_STATE,
    target: PROCESS_POWER_THROTTLING_STATE,
}
impl PowerExperiment {
    fn start(process: platform::ControlProcess, timer: bool) -> Result<Self> {
        let original =
            power_policy(&process).map_err(|_| "Experimental policy capture unavailable")?;
        let mut target = original;
        if timer {
            target.ControlMask |= PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION;
            target.StateMask |= PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION;
        } else {
            target.ControlMask |= PROCESS_POWER_THROTTLING_EXECUTION_SPEED;
            target.StateMask &= !PROCESS_POWER_THROTTLING_EXECUTION_SPEED;
        }
        let experiment = Self {
            process,
            original,
            target,
        };
        if !set_power(&experiment.process, &target) || !experiment.matches() {
            return Err("Experimental policy denied or not observed; no benchmark result");
        }
        Ok(experiment)
    }
    fn matches(&self) -> bool {
        power_policy(&self.process).is_ok_and(|p| same_power(&p, &self.target))
    }
    fn restore(&self) -> Result<()> {
        if unsafe { WaitForSingleObject(self.process.raw(), 0) } != WAIT_TIMEOUT {
            return Ok(());
        }
        if power_policy(&self.process).is_ok_and(|p| same_power(&p, &self.original)) {
            return Ok(());
        }
        if set_power(&self.process, &self.original)
            && power_policy(&self.process).is_ok_and(|p| same_power(&p, &self.original))
        {
            Ok(())
        } else {
            Err("Experimental power-policy restoration failed")
        }
    }
}
impl Drop for PowerExperiment {
    fn drop(&mut self) {
        if self.restore().is_err() {
            eprintln!("Experimental power-policy cleanup failed; verify the dedicated test client");
        }
    }
}

struct Run {
    api: Api,
    config: Config,
    original: Vec<Account>,
    expected: Vec<Account>,
    windows: Vec<HWND>,
    samplers: Vec<Managed>,
    priorities: Vec<u32>,
    affinities: Vec<usize>,
    powers: Vec<PROCESS_POWER_THROTTLING_STATE>,
    memories: Vec<u32>,
    manual_window: Option<(usize, WINDOWPLACEMENT)>,
    power_experiment: Option<PowerExperiment>,
    journal: Journal,
}
impl Run {
    async fn start(&mut self) -> Result<()> {
        let mut prepared = Vec::new();
        for original in &self.original {
            let mut a = original.clone();
            a.client.mode = ClientMode::Normal;
            a.client.auto_hide = false;
            a.client.start_on_open = false;
            a.client.monitoring = true;
            a.resource = ResourceSettings::default();
            a.anti_afk.enabled = false;
            a.auto_recovery = false;
            prepared.push(a.clone());
            self.api.patch(&a).await?;
            self.api.action(&a.id, "start").await?;
        }
        let until = Instant::now() + Duration::from_secs(180);
        loop {
            let snapshot = self.api.snapshot().await?;
            self.environment(&snapshot)?;
            let accounts = selected(&snapshot, &self.config.account_ids)?;
            if accounts.iter().all(|a| {
                a.status == crate::model::Status::Running
                    && a.connected_since.is_some()
                    && a.process.is_some()
            }) {
                self.expected = accounts.into_iter().cloned().collect();
                for (a, intended) in self.expected.iter().zip(&prepared) {
                    if a.client != intended.client
                        || a.resource != intended.resource
                        || a.anti_afk != intended.anti_afk
                        || a.auto_recovery != intended.auto_recovery
                        || a.target != intended.target
                    {
                        return Err("Launch changed selected per-account settings");
                    }
                    let identity = a.process.as_ref().ok_or("Process missing")?;
                    let handle = platform::verify_controls(identity, a.generation)
                        .map_err(|_| "Launch ownership not verified")?;
                    let mut affinity = 0;
                    let mut system = 0;
                    let priority = unsafe { GetPriorityClass(handle.raw()) };
                    if priority == 0
                        || unsafe {
                            GetProcessAffinityMask(handle.raw(), &mut affinity, &mut system)
                        } == 0
                    {
                        return Err("Original priority/affinity capture failed");
                    }
                    self.powers.push(
                        power_policy(&handle)
                            .map_err(|_| "Original EcoQoS policy capture failed")?,
                    );
                    self.memories.push(
                        memory_priority(&handle)
                            .map_err(|_| "Original memory priority capture failed")?,
                    );
                    self.priorities.push(priority);
                    self.affinities.push(affinity);
                    self.windows.push(
                        player_window(identity.pid)
                            .map_err(|_| "Unique visible player window unavailable")?,
                    );
                    self.samplers.push(Managed::new(identity.clone()));
                    self.journal
                        .event(json!({"event":"process", "account_id":a.id,
                        "pid":identity.pid,"player_version":process_version(&handle)}))?;
                }
                return Ok(());
            }
            if Instant::now() >= until {
                return Err(
                    "Launch/connection confirmation timed out; no Connected result claimed",
                );
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
    fn environment(&self, snapshot: &Snapshot) -> Result<()> {
        if snapshot.session_suspended || snapshot.network_suspended || suspended() {
            return Err("Desktop lock/sleep or network suspension interrupted the run");
        }
        Ok(())
    }
    async fn steady(&self) -> Result<Snapshot> {
        let snapshot = self.api.snapshot().await?;
        self.environment(&snapshot)?;
        for (expected, actual) in self
            .expected
            .iter()
            .zip(selected(&snapshot, &self.config.account_ids)?)
        {
            unchanged(expected, actual)?;
        }
        Ok(snapshot)
    }
    async fn transition(
        &mut self,
        index: usize,
        mode: ClientMode,
        profile: ResourceMode,
        window: i32,
    ) -> Result<()> {
        self.restore_experiment()?;
        self.steady().await?;
        self.restore_manual_window()?;
        let peer = 1 - index;
        let peer_show = show_command(self.windows[peer]).map_err(|_| "Peer window unavailable")?;
        let mut next = self.expected[index].clone();
        next.client.mode = mode;
        next.resource.mode = profile;
        let requested = Instant::now();
        self.api.patch(&next).await?;
        self.expected[index] = next;
        let identity = self.expected[index]
            .process
            .as_ref()
            .ok_or("Process missing")?;
        let process = platform::verify_controls(identity, self.expected[index].generation)
            .map_err(|_| "Transition ownership verification failed")?;
        let until = requested + Duration::from_secs(10);
        let mut native_ms = None;
        loop {
            let snapshot = self.steady().await?;
            if unsafe { WaitForSingleObject(process.raw(), 0) } != WAIT_TIMEOUT
                || !window_owned(self.windows[index], identity.pid)
                || !window_matches(self.windows[peer], peer_show)
            {
                return Err("Transition changed process/window ownership or peer window state");
            }
            if window_matches(self.windows[index], window) {
                native_ms.get_or_insert_with(|| requested.elapsed().as_secs_f64() * 1000.0);
                let runtime = snapshot.runtime_controls.get(&self.expected[index].id);
                let priority = unsafe { GetPriorityClass(process.raw()) };
                let mut affinity = 0;
                let mut system = 0;
                if unsafe { GetProcessAffinityMask(process.raw(), &mut affinity, &mut system) } == 0
                    || affinity != self.affinities[index]
                {
                    return Err("Original affinity was not preserved");
                }
                if runtime.and_then(|r| r.get("client_mode")) == Some(&json!(mode))
                    && priority
                        == if profile == ResourceMode::Normal && mode == ClientMode::Normal
                            || self.priorities[index] == IDLE_PRIORITY_CLASS
                        {
                            self.priorities[index]
                        } else {
                            BELOW_NORMAL_PRIORITY_CLASS
                        }
                {
                    let power = power_policy(&process)
                        .map_err(|_| "EcoQoS policy measurement unavailable")?;
                    let memory = memory_priority(&process)
                        .map_err(|_| "Memory priority measurement unavailable")?;
                    let efficient =
                        mode == ClientMode::Headless || profile == ResourceMode::LowResource;
                    let target_power = if efficient {
                        background_power(self.powers[index])
                    } else {
                        self.powers[index]
                    };
                    let target_memory = if efficient {
                        self.memories[index].min(MEMORY_PRIORITY_LOW)
                    } else {
                        self.memories[index]
                    };
                    if !same_power(&power, &target_power) || memory != target_memory {
                        return Err("Efficiency policy application/restoration was not verified");
                    }
                    if mode == ClientMode::Headless && unsafe { IsIconic(self.windows[index]) } == 0
                    {
                        return Err("Headless window is hidden without verified minimization");
                    }
                    return self.journal.event(
                        json!({"event":"transition", "account_id": self.expected[index].id,
                        "mode": mode, "profile": profile, "native_window_ms": native_ms,
                        "backend_confirmed_ms": requested.elapsed().as_secs_f64() * 1000.0,
                        "continuity":"backend_unchanged_server_export_required"}),
                    );
                }
            }
            if Instant::now() >= until {
                return Err("Window/mode/profile confirmation exceeded ten seconds");
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    async fn sample(&mut self, label: &str, strict: bool) -> Result<()> {
        if self.power_experiment.as_ref().is_some_and(|p| !p.matches()) {
            return Err("Experimental policy changed during capture");
        }
        let snapshot = if strict {
            self.steady().await?
        } else {
            self.api.snapshot().await?
        };
        self.environment(&snapshot)?;
        let mut rows = Vec::new();
        let mut pids = Vec::new();
        for (i, account) in selected(&snapshot, &self.config.account_ids)?
            .into_iter()
            .enumerate()
        {
            let mut row = public_account(account, snapshot.runtime_controls.get(&account.id));
            if account.client != self.expected[i].client
                || account.resource != self.expected[i].resource
                || account.auto_recovery != self.expected[i].auto_recovery
                || account.anti_afk != self.expected[i].anti_afk
                || account.target != self.expected[i].target
            {
                return Err("Observation changed saved account preferences");
            }
            if let Some(identity) = &account.process {
                if self.samplers[i].identity() != identity {
                    self.samplers[i] = Managed::new(identity.clone());
                    // The production controller may already have hidden a rejoined window.
                    self.windows[i] = player_window(identity.pid).unwrap_or(null_mut());
                    let version = platform::verify_controls(identity, account.generation)
                        .ok()
                        .and_then(|handle| process_version(&handle));
                    self.journal
                        .event(json!({"event":"process", "account_id":account.id,
                        "pid":identity.pid,"player_version":version}))?;
                }
                match native_sample(&mut self.samplers[i], account, self.windows[i]) {
                    Ok(native) => {
                        if strict
                            && account.client.mode == ClientMode::Normal
                            && account.resource.mode == ResourceMode::Normal
                            && (native["priority"] != json!(self.priorities[i])
                                || native["power_control_mask"]
                                    != json!(self.powers[i].ControlMask)
                                || native["power_state_mask"] != json!(self.powers[i].StateMask)
                                || native["memory_priority"] != json!(self.memories[i]))
                        {
                            return Err("Normal account process policy changed");
                        }
                        row["native"] = native;
                        pids.push(identity.pid);
                    }
                    Err(error) if strict => return Err(error),
                    Err(_) => {
                        row["native_unavailable"] = json!(
                            "Process exited or ownership verification unavailable during recovery"
                        )
                    }
                }
            }
            rows.push(row);
        }
        self.journal
            .event(json!({"event":"sample", "phase":label, "accounts":rows,
            "gpu_engines":gpu_samples(&pids)}))
    }
    async fn sample_for(&mut self, seconds: u64, label: &str, strict: bool) -> Result<()> {
        let start = Instant::now();
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        while start.elapsed() < Duration::from_secs(seconds) {
            tick.tick().await;
            self.sample(label, strict).await?;
        }
        Ok(())
    }
    async fn switches(&mut self) -> Result<()> {
        self.sample("switches", true).await?;
        for i in 0..2 {
            for _ in 0..10 {
                self.transition(i, ClientMode::Headless, ResourceMode::Normal, SW_HIDE)
                    .await?;
                self.transition(i, ClientMode::Normal, ResourceMode::Normal, SW_SHOWNORMAL)
                    .await?;
                self.sample("switches", true).await?;
            }
        }
        self.journal.event(json!({"event":"result", "case":"switches",
            "status":"UNVERIFIED", "detail":"OS/backend checks passed; independent server export and gameplay confirmation still required"}))
    }
    async fn benchmark(&mut self) -> Result<()> {
        let conditions = [
            (
                "normal",
                ClientMode::Normal,
                ResourceMode::Normal,
                SW_SHOWNORMAL,
            ),
            (
                "minimized",
                ClientMode::Normal,
                ResourceMode::Normal,
                SW_MINIMIZE,
            ),
            ("hidden", ClientMode::Normal, ResourceMode::Normal, SW_HIDE),
            (
                "headless",
                ClientMode::Headless,
                ResourceMode::Normal,
                SW_HIDE,
            ),
            (
                "balanced",
                ClientMode::Normal,
                ResourceMode::Balanced,
                SW_SHOWNORMAL,
            ),
            (
                "low_resource",
                ClientMode::Normal,
                ResourceMode::LowResource,
                SW_MINIMIZE,
            ),
            (
                "headless_ecoqos_off",
                ClientMode::Headless,
                ResourceMode::Normal,
                SW_HIDE,
            ),
            (
                "headless_ignore_timer",
                ClientMode::Headless,
                ResourceMode::Normal,
                SW_HIDE,
            ),
        ];
        for i in 0..2 {
            for block in 0..3 {
                self.transition(i, ClientMode::Normal, ResourceMode::Normal, SW_SHOWNORMAL)
                    .await?;
                let checkpoint = self.config.output.join(format!("continue-{i}-{block}"));
                self.journal
                    .event(json!({"event":"checkpoint", "account_index":i, "block":block}))?;
                println!(
                    "Arrange both game windows visibly. Interact normally in BOTH accounts, then create {} within five minutes.",
                    checkpoint.display()
                );
                let until = Instant::now() + Duration::from_secs(300);
                while !checkpoint.is_file() {
                    self.steady().await?;
                    if Instant::now() >= until {
                        return Err("Manual benchmark checkpoint timed out");
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                for step in 0..conditions.len() {
                    let (name, mode, profile, show) =
                        conditions[(step + block * 2) % conditions.len()];
                    self.transition(i, ClientMode::Normal, ResourceMode::Normal, SW_SHOWNORMAL)
                        .await?;
                    if name == "minimized" || name == "hidden" {
                        let identity =
                            self.expected[i].process.as_ref().ok_or("Process missing")?;
                        let _handle =
                            platform::verify_controls(identity, self.expected[i].generation)
                                .map_err(|_| "Window baseline ownership failed")?;
                        let mut placement: WINDOWPLACEMENT = unsafe { std::mem::zeroed() };
                        placement.length = size_of::<WINDOWPLACEMENT>() as u32;
                        if unsafe { GetWindowPlacement(self.windows[i], &mut placement) } == 0 {
                            return Err("Pure window placement capture failed");
                        }
                        self.manual_window = Some((i, placement));
                        if unsafe { ShowWindowAsync(self.windows[i], show) } == 0 {
                            return Err("Pure window request denied");
                        }
                        let until = Instant::now() + Duration::from_secs(10);
                        while !window_matches(self.windows[i], show) {
                            self.steady().await?;
                            if Instant::now() >= until {
                                return Err("Pure window confirmation timed out");
                            }
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                    } else {
                        self.transition(i, mode, profile, show).await?;
                    }
                    if matches!(name, "headless_ecoqos_off" | "headless_ignore_timer") {
                        let account = &self.expected[i];
                        let process = platform::verify_controls(
                            account.process.as_ref().ok_or("Process missing")?,
                            account.generation,
                        )
                        .map_err(|_| "Experimental policy ownership failed")?;
                        self.power_experiment = Some(PowerExperiment::start(
                            process,
                            name == "headless_ignore_timer",
                        )?);
                    }
                    let label = format!("a{i}-b{block}-{name}");
                    self.sample_for(30, &format!("settle-{label}"), true)
                        .await?;
                    let pid = self.expected[i]
                        .process
                        .as_ref()
                        .ok_or("Process missing")?
                        .pid;
                    let mut capture = capture(&self.config, pid, &label)?;
                    let peer_pid = self.expected[1 - i]
                        .process
                        .as_ref()
                        .ok_or("Peer process missing")?
                        .pid;
                    let mut peer_capture = super::acceptance::capture(
                        &self.config,
                        peer_pid,
                        &format!("{label}-peer"),
                    )?;
                    self.sample_for(60, &label, true).await?;
                    if capture.is_some() || peer_capture.is_some() {
                        // Allow the console collector to flush its final CSV rows.
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                    for capture in [&mut capture, &mut peer_capture].into_iter().flatten() {
                        if !matches!(capture.child.try_wait(), Ok(Some(status)) if status.success())
                        {
                            return Err(
                                "PresentMon did not complete successfully; frame evidence invalid",
                            );
                        }
                    }
                }
            }
            self.transition(i, ClientMode::Normal, ResourceMode::Normal, SW_SHOWNORMAL)
                .await?;
        }
        Ok(())
    }
    fn restore_experiment(&mut self) -> Result<()> {
        if let Some(experiment) = &self.power_experiment {
            experiment.restore()?;
        }
        self.power_experiment = None;
        Ok(())
    }
    async fn activity_observation(&mut self, seconds: u64) -> Result<()> {
        for i in 0..2 {
            self.transition(i, ClientMode::Headless, ResourceMode::Normal, SW_HIDE)
                .await?;
            self.expected[i].auto_recovery = false;
            self.expected[i].anti_afk.enabled = true;
            self.expected[i].anti_afk.headless_foreground_activity = true;
            self.api.patch(&self.expected[i]).await?;
        }
        self.sample_for(
            seconds,
            if seconds == 1500 {
                "activity25"
            } else {
                "activity60"
            },
            true,
        )
        .await
    }
    async fn observe(&mut self, recovery: bool) -> Result<()> {
        for i in 0..2 {
            self.transition(
                i,
                ClientMode::Headless,
                if i == 0 {
                    ResourceMode::Balanced
                } else {
                    ResourceMode::LowResource
                },
                SW_HIDE,
            )
            .await?;
            self.expected[i].auto_recovery = recovery;
            self.api.patch(&self.expected[i]).await?;
        }
        self.sample_for(
            if recovery { 3600 } else { 1500 },
            if recovery { "idle60" } else { "idle25" },
            false,
        )
        .await
    }
    async fn recovery(&mut self, terminate: bool) -> Result<()> {
        self.expected[0].auto_recovery = true;
        self.api.patch(&self.expected[0]).await?;
        self.transition(0, ClientMode::Headless, ResourceMode::Balanced, SW_HIDE)
            .await?;
        let identity = self.expected[0].process.clone().ok_or("Process missing")?;
        let _verified = platform::verify_controls(&identity, self.expected[0].generation)
            .map_err(|_| "Termination ownership failed")?;
        if terminate {
            // Existing close() revalidates creation time and tracker before forced termination.
            platform::close(&identity, true)
                .map_err(|_| "Verified test-process termination failed")?;
        } else {
            println!(
                "Using your owned experience's server controls, disconnect only test account {} within five minutes.",
                self.expected[0].id
            );
        }
        let until = Instant::now() + Duration::from_secs(300);
        loop {
            self.sample(
                if terminate {
                    "forced_recovery"
                } else {
                    "server_disconnect"
                },
                false,
            )
            .await?;
            let snapshot = self.api.snapshot().await?;
            let accounts = selected(&snapshot, &self.config.account_ids)?;
            unchanged(&self.expected[1], accounts[1])?;
            let recovered = accounts[0];
            if recovered.status == crate::model::Status::Running
                && recovered.process.as_ref().is_some_and(|p| p != &identity)
                && recovered.connected_since.is_some()
                && recovered.connected_since != self.expected[0].connected_since
                && recovered.reconnection_attempts > self.expected[0].reconnection_attempts
            {
                if recovered.client != self.expected[0].client
                    || recovered.resource != self.expected[0].resource
                {
                    return Err("Recovery lost selected provider/mode/profile");
                }
                if snapshot
                    .runtime_controls
                    .get(&recovered.id)
                    .and_then(|r| r.get("client_mode"))
                    == Some(&json!("headless"))
                {
                    return self.journal.event(json!({"event":"recovery", "status":"UNVERIFIED",
                        "detail":"Replacement and Headless observed; server export required; this is a new connection"}));
                }
            }
            if Instant::now() >= until {
                return Err("Recovery confirmation timed out");
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
    async fn isolation(&mut self) -> Result<()> {
        self.transition(0, ClientMode::Headless, ResourceMode::Balanced, SW_HIDE)
            .await?;
        self.transition(
            1,
            ClientMode::Normal,
            ResourceMode::LowResource,
            SW_MINIMIZE,
        )
        .await?;
        for action in ["restart", "stop", "start"] {
            let previous = self.expected[0].process.clone();
            self.api.action(&self.expected[0].id, action).await?;
            let until = Instant::now() + Duration::from_secs(180);
            loop {
                self.sample("isolation", false).await?;
                let snapshot = self.api.snapshot().await?;
                let accounts = selected(&snapshot, &self.config.account_ids)?;
                unchanged(&self.expected[1], accounts[1])?;
                if !window_matches(self.windows[1], SW_MINIMIZE) {
                    return Err("Peer window changed during isolation test");
                }
                let account = accounts[0];
                let done = if action == "stop" {
                    !account.desired_running
                        && account.process.is_none()
                        && account.tracker.is_none()
                } else {
                    account.status == crate::model::Status::Running
                        && account.process.is_some()
                        && account.process != previous
                        && account.connected_since.is_some()
                        && snapshot
                            .runtime_controls
                            .get(&account.id)
                            .and_then(|r| r.get("client_mode"))
                            == Some(&json!("headless"))
                };
                if done {
                    self.expected[0] = account.clone();
                    self.journal.event(json!({"event":"isolation_action", "action":action, "backend":"verified_peer_unchanged", "server_export":"required"}))?;
                    break;
                }
                if Instant::now() >= until {
                    return Err("Isolation action confirmation timed out");
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
        Ok(())
    }
    async fn occlusion(&mut self) -> Result<()> {
        let account = self.expected[0].clone();
        let identity = account.process.as_ref().ok_or("Process missing")?;
        for phase in ["visible_before", "occluded", "visible_after"] {
            let label = format!("a0-{phase}");
            self.steady().await?;
            let _handle = platform::verify_controls(identity, account.generation)
                .map_err(|_| "Occlusion ownership failed")?;
            let _cover = if phase == "occluded" {
                Some(Occluder::new(self.windows[0], identity.pid)?)
            } else {
                None
            };
            self.sample_for(30, &format!("settle-{label}"), true)
                .await?;
            let mut collector = capture(&self.config, identity.pid, &label)?;
            self.sample_for(60, &label, true).await?;
            if let Some(collector) = &mut collector {
                tokio::time::sleep(Duration::from_secs(2)).await;
                if !matches!(collector.child.try_wait(), Ok(Some(status)) if status.success()) {
                    return Err("Occlusion frame capture did not complete");
                }
            }
        }
        Ok(())
    }
    async fn cleanup(&mut self) -> Result<()> {
        // Stop still runs if an independently changed identity prevents placement restoration.
        let _ = self.restore_manual_window();
        let mut ok = self.restore_experiment().is_ok();
        for original in &self.original {
            ok &= self.api.action(&original.id, "stop").await.is_ok();
        }
        let until = Instant::now() + Duration::from_secs(120);
        loop {
            if let Ok(snapshot) = self.api.snapshot().await
                && let Ok(accounts) = selected(&snapshot, &self.config.account_ids)
                && accounts
                    .iter()
                    .all(|a| !a.desired_running && a.process.is_none() && a.tracker.is_none())
            {
                for original in &self.original {
                    ok &= self.api.patch(original).await.is_ok();
                }
                return if ok {
                    Ok(())
                } else {
                    Err("Cleanup request failed; verify account preferences in UI")
                };
            }
            if Instant::now() >= until {
                return Err(
                    "Cleanup could not confirm Stop; preferences not restored; inspect dedicated accounts",
                );
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
    fn restore_manual_window(&mut self) -> Result<()> {
        if let Some((i, placement)) = self.manual_window {
            let account = &self.expected[i];
            let identity = account.process.as_ref().ok_or("Process missing")?;
            let _handle = platform::verify_controls(identity, account.generation)
                .map_err(|_| "Pure window restoration ownership failed")?;
            if !window_owned(self.windows[i], identity.pid)
                || unsafe { SetWindowPlacement(self.windows[i], &placement) } == 0
            {
                return Err("Pure window placement restoration failed");
            }
            self.manual_window = None;
        }
        Ok(())
    }
}

fn process_version(process: &platform::ControlProcess) -> Option<String> {
    use windows_sys::Win32::Storage::FileSystem::*;
    unsafe {
        let mut path = vec![0u16; 32768];
        let mut length = path.len() as u32;
        if QueryFullProcessImageNameW(process.raw(), 0, path.as_mut_ptr(), &mut length) == 0 {
            return None;
        }
        let size = GetFileVersionInfoSizeW(path.as_ptr(), null_mut());
        if size == 0 || size > 1024 * 1024 {
            return None;
        }
        let mut data = vec![0u8; size as usize];
        if GetFileVersionInfoW(path.as_ptr(), 0, size, data.as_mut_ptr().cast()) == 0 {
            return None;
        }
        let mut info = null_mut();
        let mut bytes = 0;
        if VerQueryValueW(
            data.as_ptr().cast(),
            platform::wide("\\").as_ptr(),
            &mut info,
            &mut bytes,
        ) == 0
            || info.is_null()
            || bytes < size_of::<VS_FIXEDFILEINFO>() as u32
        {
            return None;
        }
        let version = info.cast::<VS_FIXEDFILEINFO>().read_unaligned();
        if version.dwSignature != 0xfeef04bd {
            return None;
        }
        Some(format!(
            "{}.{}.{}.{}",
            version.dwFileVersionMS >> 16,
            version.dwFileVersionMS & 0xffff,
            version.dwFileVersionLS >> 16,
            version.dwFileVersionLS & 0xffff
        ))
    }
}

struct Occluder(HWND);
impl Occluder {
    fn new(player: HWND, pid: u32) -> Result<Self> {
        let mut rect: RECT = unsafe { std::mem::zeroed() };
        if !window_owned(player, pid)
            || !window_matches(player, SW_SHOWNORMAL)
            || unsafe { GetWindowRect(player, &mut rect) } == 0
        {
            return Err("Occlusion requires the verified visible player window");
        }
        let cover = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                platform::wide("STATIC").as_ptr(),
                platform::wide("RoLauncher authorized occlusion test").as_ptr(),
                WS_POPUP | WS_VISIBLE | 4, // SS_BLACKRECT paints an opaque STATIC control.
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                null_mut(),
                null_mut(),
                GetModuleHandleW(null()),
                null(),
            )
        };
        if cover.is_null() {
            return Err("Occlusion cover creation failed");
        }
        unsafe {
            windows_sys::Win32::Graphics::Gdi::UpdateWindow(cover);
        }
        Ok(Self(cover))
    }
}
impl Drop for Occluder {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.0);
        }
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "Live authorized Roblox acceptance only; use scripts/client-acceptance.ps1"]
async fn live_roblox_acceptance() {
    let result = execute().await;
    assert!(result.is_ok(), "{}", result.err().unwrap_or(""));
}

#[test]
#[ignore = "Interactive owned STATIC-window occlusion fixture; not a Roblox/rendering test"]
fn opaque_cover_is_owned_reversible_and_does_not_hide_the_target() {
    assert!(!suspended());
    struct Window(HWND);
    impl Drop for Window {
        fn drop(&mut self) {
            unsafe {
                DestroyWindow(self.0);
            }
        }
    }
    let window = Window(unsafe {
        CreateWindowExW(
            0,
            platform::wide("STATIC").as_ptr(),
            platform::wide("RoLauncher isolated occlusion fixture").as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            100,
            100,
            400,
            240,
            null_mut(),
            null_mut(),
            GetModuleHandleW(null()),
            null(),
        )
    });
    assert!(!window.0.is_null());
    let pid = unsafe { GetCurrentProcessId() };
    assert!(Occluder::new(window.0, pid.wrapping_add(1)).is_err());
    let cover = Occluder::new(window.0, pid).unwrap();
    let mut target_rect: RECT = unsafe { std::mem::zeroed() };
    let mut cover_rect = target_rect;
    assert_ne!(unsafe { GetWindowRect(window.0, &mut target_rect) }, 0);
    assert_ne!(unsafe { GetWindowRect(cover.0, &mut cover_rect) }, 0);
    assert_eq!(
        (
            target_rect.left,
            target_rect.top,
            target_rect.right,
            target_rect.bottom
        ),
        (
            cover_rect.left,
            cover_rect.top,
            cover_rect.right,
            cover_rect.bottom
        )
    );
    assert!(window_matches(window.0, SW_SHOWNORMAL));
    let handle = cover.0;
    drop(cover);
    assert_eq!(unsafe { IsWindow(handle) }, 0);
    assert!(window_matches(window.0, SW_SHOWNORMAL));
}

async fn execute() -> Result<()> {
    let path =
        std::env::var_os("ROLAUNCHER_ACCEPTANCE_CONFIG").ok_or("Acceptance config missing")?;
    let config: Config =
        serde_json::from_slice(&std::fs::read(path).map_err(|_| "Cannot read config")?)
            .map_err(|_| "Invalid acceptance config")?;
    config.validate()?;
    let token = zeroize::Zeroizing::new(
        std::env::var("ROLAUNCHER_API_TOKEN")
            .map_err(|_| "API token missing; use secure wrapper prompt")?,
    );
    if token.is_empty() {
        return Err("API token is empty");
    }
    let api = Api {
        client: reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|_| "Cannot construct local API client")?,
        base: config.base_url.clone(),
        token,
    };
    let snapshot = api.snapshot().await?;
    let original: Vec<Account> = selected(&snapshot, &config.account_ids)?
        .into_iter()
        .cloned()
        .collect();
    for a in &original {
        if a.desired_running
            || a.process.is_some()
            || a.tracker.is_some()
            || a.status != crate::model::Status::Stopped
        {
            return Err("Both dedicated accounts must be fully stopped before testing");
        }
        if a.target.as_ref().map(|t| t.place_id) != Some(config.place_id) {
            return Err("Both saved targets must match the explicitly authorized place");
        }
        if !matches!(
            a.client.bootstrapper,
            Bootstrapper::Official | Bootstrapper::Bloxstrap | Bootstrapper::Fishstrap
        ) {
            return Err("Select Official, Bloxstrap or Fishstrap explicitly in each account");
        }
        a.client.bootstrapper.request(&a.client).map_err(
            |_| "Selected provider preflight failed; no fallback or registry changes attempted",
        )?;
    }
    // create_dir, not create_dir_all: refuse existing run directories and evidence overwrite.
    std::fs::create_dir(&config.output)
        .map_err(|_| "Create a fresh output directory under an existing parent")?;
    let file = File::options()
        .write(true)
        .create_new(true)
        .open(config.output.join("journal.jsonl"))
        .map_err(|_| "Cannot create evidence journal")?;
    let mut run = Run {
        api,
        config,
        original,
        expected: Vec::new(),
        windows: Vec::new(),
        samplers: Vec::new(),
        priorities: Vec::new(),
        affinities: Vec::new(),
        powers: Vec::new(),
        memories: Vec::new(),
        manual_window: None,
        power_experiment: None,
        journal: Journal(file),
    };
    run.journal.event(json!({"event":"begin", "version":env!("CARGO_PKG_VERSION"), "run_id":run.config.output.file_name().and_then(|n| n.to_str()),
        "scenario":format!("{:?}", run.config.scenario), "place_id":run.config.place_id,
        "frame_capture":run.config.presentmon.is_some(), "server_evidence":"manual_export_required"}))?;
    let work = async {
        run.start().await?;
        match run.config.scenario {
            Scenario::Switches => run.switches().await,
            Scenario::Benchmark => run.benchmark().await,
            Scenario::Idle25 => run.observe(false).await,
            Scenario::Idle60 => run.observe(true).await,
            Scenario::Activity25 => run.activity_observation(1500).await,
            Scenario::Activity60 => run.activity_observation(3600).await,
            Scenario::Recovery => run.recovery(true).await,
            Scenario::ServerDisconnect => run.recovery(false).await,
            Scenario::Isolation => run.isolation().await,
            Scenario::Occlusion => run.occlusion().await,
        }
    };
    let result = tokio::select! { result = work => result, _ = tokio::signal::ctrl_c() => Err("Run interrupted; cleanup requested") };
    let cleanup = run.cleanup().await;
    run.journal.event(json!({"event":"end", "status":if result.is_ok() && cleanup.is_ok() {"UNVERIFIED"} else {"FAIL"},
        "error":result.as_ref().err(), "cleanup_error":cleanup.as_ref().err(),
        "detail":"Live pass requires independent server export, frame evidence and manual gameplay confirmation"}))?;
    result.and(cleanup)
}

#[test]
#[ignore = "Isolated Windows process-policy fixture; no Roblox or input; run with --test-threads=1"]
fn native_power_experiments_restore_exact_policy() {
    let identity = platform::control_fixture_identity(uuid::Uuid::new_v4());
    let process = platform::verify_controls(&identity, identity.generation).unwrap();
    let original = power_policy(&process).unwrap();
    for timer in [false, true] {
        let experiment = PowerExperiment::start(process.duplicate().unwrap(), timer).unwrap();
        assert!(experiment.matches());
        experiment.restore().unwrap();
        drop(experiment);
        assert!(same_power(&power_policy(&process).unwrap(), &original));
    }
    assert!(PowerExperiment::start(platform::control_fixture_query_only(), true).is_err());
    assert!(same_power(&power_policy(&process).unwrap(), &original));
}

#[test]
fn rejects_untrusted_endpoints_duplicate_accounts_and_missing_authorization() {
    let mut c: Config = serde_json::from_value(json!({"base_url":"http://127.0.0.1:38471/",
        "account_ids":["1","2"],"place_id":1,"scenario":"switches",
        "output":"C:\\acceptance-run", "presentmon":null,"authorized_test_experience":true}))
    .unwrap();
    assert!(c.validate().is_ok());
    for endpoint in [
        "http://localhost:38471",
        "http://127.0.0.1:38471/?token=x",
        "http://127.0.0.1:38471@evil.test",
        "https://127.0.0.1:38471",
        "http://127.0.0.1:38471/v1",
    ] {
        c.base_url = endpoint.into();
        assert!(c.validate().is_err());
    }
    c.base_url = "http://127.0.0.1:38471/".into();
    c.account_ids[1] = "1".into();
    assert!(c.validate().is_err());
    c.account_ids[1] = "2".into();
    c.authorized_test_experience = false;
    assert!(c.validate().is_err());
}

#[test]
fn continuity_and_export_do_not_confuse_process_uptime_with_connection() {
    let mut a = Account::new("1".into(), "fixture".into());
    a.status = crate::model::Status::Running;
    a.process = Some(platform::control_fixture_identity(a.generation));
    a.connected_since = Some(chrono::Utc::now());
    assert!(unchanged(&a, &a).is_ok());
    let mut b = a.clone();
    b.connected_since = Some(chrono::Utc::now() + chrono::Duration::seconds(1));
    assert!(unchanged(&a, &b).is_err());
    b = a.clone();
    b.process.as_mut().unwrap().creation_time = "0".into();
    assert!(unchanged(&a, &b).is_err());
    b = a.clone();
    b.client.bootstrapper = Bootstrapper::Fishstrap;
    assert!(unchanged(&a, &b).is_err());
    let exported = public_account(&a, None).to_string();
    assert!(!exported.contains(&a.process.as_ref().unwrap().tracker));
    assert!(!exported.contains("fixture"));
}

#[tokio::test(flavor = "current_thread")]
async fn cleanup_stops_only_selected_accounts_before_restoring_preferences() {
    use axum::{Json, Router, extract::Request};
    use std::sync::{Arc, Mutex};
    let accounts = vec![
        Account::new("1".into(), "one".into()),
        Account::new("2".into(), "two".into()),
    ];
    let calls = Arc::new(Mutex::new(Vec::new()));
    let recorded = calls.clone();
    let response_accounts = accounts.clone();
    let app = Router::new().fallback(move |request: Request| {
        let calls = recorded.clone();
        let accounts = response_accounts.clone();
        async move {
            calls.lock().unwrap().push((request.method().to_string(), request.uri().path().to_string()));
            Json(if request.uri().path() == "/v1/status" {
                json!({"accounts":accounts, "runtime_controls":{}, "session_suspended":false,"network_suspended":false})
            } else { Value::Null })
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let path = std::env::current_dir()
        .unwrap()
        .join("target")
        .join(format!("acceptance-cleanup-{}.jsonl", uuid::Uuid::new_v4()));
    let mut run = Run {
        api: Api {
            client: reqwest::Client::builder().no_proxy().build().unwrap(),
            base: base.clone(),
            token: zeroize::Zeroizing::new("FIXTURE_ONLY".into()),
        },
        config: Config {
            base_url: base,
            account_ids: ["1".into(), "2".into()],
            place_id: 1,
            scenario: Scenario::Switches,
            output: path.clone(),
            presentmon: None,
            authorized_test_experience: true,
        },
        original: accounts,
        expected: Vec::new(),
        windows: Vec::new(),
        samplers: Vec::new(),
        priorities: Vec::new(),
        affinities: Vec::new(),
        powers: Vec::new(),
        memories: Vec::new(),
        manual_window: None,
        journal: Journal(File::create(&path).unwrap()),
        power_experiment: None,
    };
    assert!(run.cleanup().await.is_ok());
    assert_eq!(
        *calls.lock().unwrap(),
        vec![
            ("POST".into(), "/v1/accounts/1/stop".into()),
            ("POST".into(), "/v1/accounts/2/stop".into()),
            ("GET".into(), "/v1/status".into()),
            ("PATCH".into(), "/v1/accounts/1".into()),
            ("PATCH".into(), "/v1/accounts/2".into())
        ]
    );
    server.abort();
    drop(run);
    std::fs::remove_file(path).unwrap();
}
