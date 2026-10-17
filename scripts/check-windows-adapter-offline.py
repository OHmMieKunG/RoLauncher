"""Type-check adapter Win32 calls using cached bindings; this does not build the application."""
from pathlib import Path
import os
import subprocess
import tomllib

root = Path(__file__).resolve().parent.parent
cache = Path(os.environ.get("CARGO_HOME", str(Path.home() / ".cargo"))) / "registry/src"
matches = list(cache.glob("*/windows-sys-0.61.2"))
if len(matches) != 1:
    raise SystemExit("Requires cached windows-sys 0.61.2 and windows-link 0.2.1")
bindings = matches[0]
link = bindings.parent / "windows-link-0.2.1"
out = root / "target/windows-adapter-check"
out.mkdir(parents=True, exist_ok=True)

def run(*args):
    subprocess.run(["rustc", *map(str, args)], cwd=root, check=True)

run("--edition=2021", "--crate-name", "windows_link", "--crate-type=rlib", link / "src/lib.rs", "-o", out / "libwindows_link.rlib")
manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
features = set(manifest["target"]["cfg(windows)"]["dependencies"]["windows-sys"]["features"])
catalog = tomllib.loads((bindings / "Cargo.toml").read_text(encoding="utf-8"))["features"]
pending = list(features)
while pending:
    for feature in catalog[pending.pop()]:
        if feature not in features:
            features.add(feature)
            pending.append(feature)
flags = [arg for feature in sorted(features) for arg in ["--cfg", f'feature="{feature}"']]
run("--edition=2021", "--crate-name", "windows_sys", "--crate-type=rlib", bindings / "src/lib.rs", "--extern", f"windows_link={out / 'libwindows_link.rlib'}", "-L", out, *flags, "-o", out / "libwindows_sys.rlib")
# Only the integration dependencies are substituted. The actual adapter file is compiled unchanged.
harness = '''
#![allow(dead_code)]
extern crate self as serde_json;
extern crate self as chrono;
#[derive(Clone, Copy)] pub struct Utc; pub type DateTime<T> = T; pub type Timestamp = Utc; impl Utc { pub fn now() -> Timestamp { Utc } } impl Utc { pub fn timestamp_millis(&self) -> i64 { 0 } }
#[macro_export] macro_rules! json { ($($tt:tt)*) => { String::new() } }
mod model {
    #[derive(Clone, PartialEq)] pub struct ProcessIdentity { pub pid: u32, pub generation: u64 }
    #[derive(Clone, PartialEq)] pub enum Status { Running, Stopped }
    #[derive(Clone)] pub struct Account { pub id: String, pub process: Option<ProcessIdentity>, pub generation: u64, pub desired_running: bool, pub status: Status, pub resource: crate::resource::ResourceSettings, pub anti_afk: crate::resource::AntiAfkSettings, pub client: crate::client::ClientSettings }
    #[derive(Clone)] pub struct Saved { pub account: Account }
    #[derive(Clone, Default)] pub struct Database { pub accounts: Vec<Saved> }
    #[derive(Default)] pub struct Snapshot { pub accounts: Vec<Account>, pub anti_afk_paused: bool, pub session_suspended: bool }
}
mod client {
    #[derive(Clone, Copy, PartialEq)] pub enum ClientMode { Normal, Headless, NetworkOnly }
    #[derive(Clone, PartialEq)] pub struct ClientSettings { pub mode: ClientMode, pub auto_hide: bool, pub monitoring: bool }
    impl ClientSettings { pub fn validate(&self) -> Result<(), String> { Ok(()) } }
}
mod resource {
    #[derive(Clone, Copy, PartialEq)] pub enum ResourceMode { Normal, Balanced, LowResource }
    #[derive(Clone, PartialEq)] pub struct ResourceSettings { pub mode: ResourceMode, pub affinity: Option<String> }
    impl ResourceSettings { pub fn validate(&self) -> Result<(), String> { Ok(()) } pub fn effective_mode(&self, mode: crate::client::ClientMode) -> ResourceMode { if mode == crate::client::ClientMode::Headless { ResourceMode::LowResource } else { self.mode.clone() } } pub fn affinity_mask(&self) -> Result<Option<usize>, String> { Ok(None) } }
    #[derive(Clone, Copy, PartialEq)] pub enum IdleAction { Space, Camera }
    #[derive(Clone, PartialEq)] pub struct AntiAfkSettings { pub enabled: bool, pub action: IdleAction, pub headless_foreground_activity: bool }
    pub const ANTI_AFK_SECONDS: u64 = 600;
    #[derive(Clone, Default)] pub struct RuntimeControls { pub identity: Option<crate::model::ProcessIdentity>, pub sampled_at: Option<u64>, pub cpu_percent: Option<f64>, pub working_set_bytes: Option<u64>, pub private_bytes: Option<u64>, pub page_faults_per_second: Option<f64>, pub last_activity_attempt_at: Option<crate::DateTime<crate::Utc>>, pub last_input_submitted_at: Option<crate::DateTime<crate::Utc>>, pub gpu_percent: Option<f64>, pub gpu_engine: Option<String>, pub unavailable: Vec<String>, pub resource_result: String, pub input_result: String, pub client_mode: Option<crate::client::ClientMode>, pub window_state: String, pub process_uptime_seconds: Option<u64>, pub roblox_acceptance: &'static str }
}
mod platform {
    use windows_sys::Win32::Foundation::HANDLE;
    pub struct ControlProcess;
    impl ControlProcess { pub fn duplicate(&self) -> Result<Self, String> { Err("type-check only".into()) } pub fn raw(&self) -> HANDLE { std::ptr::null_mut() } }
    pub fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() }
    pub fn verify_controls(_: &crate::model::ProcessIdentity, _: u64) -> Result<ControlProcess, String> { Err("type-check only".into()) }
    pub fn alive(_: &crate::model::ProcessIdentity) -> Result<bool, String> { Ok(false) }
    pub fn gpu_engines() -> Result<std::collections::HashMap<u32, (f64, String)>, String> { Err("type-check only".into()) }
}
#[path = "POLICY"] mod control_policy;
#[path = "ADAPTER"] mod controls;
mod engine {
    use crate::model::*;
    use std::{collections::{HashMap, BTreeMap}, sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, time::{Instant, Duration}};
    use crate::platform;
    struct Utc; impl Utc { fn now() -> u64 { 0 } }
    struct State { database: Database, anti_afk_paused: bool, session_suspended: bool, runtime_controls: BTreeMap<String, crate::resource::RuntimeControls> }
    struct Events; impl Events { fn send(&self, _: Snapshot) -> Result<(), ()> { Ok(()) } }
    struct Inner { state: Mutex<State>, events: Events, session_suspended: AtomicBool, automation: Mutex<Option<std::thread::JoinHandle<()>>> }
    #[derive(Clone)] struct Engine(Arc<Inner>);
    impl Engine { fn is_shutdown(&self) -> bool { false } fn snapshot(&self) -> Snapshot { Snapshot::default() } fn data_directory(&self) -> &std::path::Path { std::path::Path::new("unused") } }
    fn snapshot(_: &State) -> Snapshot { Snapshot::default() }
    #[path = "AUTOMATION"] mod automation;
}
'''.replace("POLICY", (root / "src/control_policy.rs").as_posix()).replace("ADAPTER", (root / "src/controls_windows.rs").as_posix()).replace("AUTOMATION", (root / "src/engine/automation.rs").as_posix())
(out / "adapter.rs").write_text(harness, encoding="utf-8")
run("--edition=2024", "--crate-type=lib", "--emit=metadata", out / "adapter.rs", "--extern", f"windows_sys={out / 'libwindows_sys.rlib'}", "-L", out, "-o", out / "adapter.rmeta")
print("Windows adapter and automation worker types checked; integration dependencies stubbed, no linking or live actions.")
