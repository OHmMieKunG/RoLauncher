use crate::client::{ClientMode, ClientSettings};
use crate::{control_policy, model::ProcessIdentity, platform, resource::*};
use std::{
    ptr::{null, null_mut},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};
use windows_sys::Win32::{
    Foundation::*,
    System::{
        LibraryLoader::GetModuleHandleW, ProcessStatus::*, RemoteDesktop::*,
        StationsAndDesktops::*, SystemInformation::GetTickCount64, Threading::*,
    },
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

static LOCKED: AtomicBool = AtomicBool::new(false);
static SLEEPING: AtomicBool = AtomicBool::new(false);

pub(crate) fn suspended() -> bool {
    if LOCKED.load(Ordering::Acquire) || SLEEPING.load(Ordering::Acquire) {
        return true;
    }
    unsafe {
        let desktop = OpenInputDesktop(0, 0, DESKTOP_READOBJECTS);
        if desktop.is_null() {
            return true;
        }
        let mut name = [0u16; 128];
        let mut needed = 0;
        let ok = GetUserObjectInformationW(
            desktop,
            UOI_NAME,
            name.as_mut_ptr().cast(),
            size_of_val(&name) as u32,
            &mut needed,
        );
        CloseDesktop(desktop);
        ok == 0
            || String::from_utf16_lossy(
                &name[..name.iter().position(|c| *c == 0).unwrap_or(name.len())],
            ) != "Default"
    }
}

unsafe extern "system" fn environment_proc(
    hwnd: HWND,
    message: u32,
    w: WPARAM,
    l: LPARAM,
) -> LRESULT {
    match message {
        WM_WTSSESSION_CHANGE => match w as u32 {
            WTS_SESSION_LOCK => LOCKED.store(true, Ordering::Release),
            WTS_SESSION_UNLOCK => LOCKED.store(false, Ordering::Release),
            _ => {}
        },
        WM_POWERBROADCAST => match w as u32 {
            PBT_APMSUSPEND => SLEEPING.store(true, Ordering::Release),
            PBT_APMRESUMEAUTOMATIC | PBT_APMRESUMESUSPEND => {
                SLEEPING.store(false, Ordering::Release)
            }
            _ => {}
        },
        _ => {}
    }
    unsafe { DefWindowProcW(hwnd, message, w, l) }
}

pub(crate) struct Environment {
    hwnd: HWND,
    error: Option<&'static str>,
}
impl Environment {
    pub(crate) fn new() -> Self {
        unsafe {
            let class = platform::wide("RoLauncherAutomationEvents");
            let instance = GetModuleHandleW(null());
            let wc = WNDCLASSW {
                lpfnWndProc: Some(environment_proc),
                hInstance: instance,
                lpszClassName: class.as_ptr(),
                ..std::mem::zeroed()
            };
            RegisterClassW(&wc);
            // A hidden top-level window receives power broadcasts as well as session changes.
            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                class.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                null_mut(),
                null_mut(),
                instance,
                null(),
            );
            let mut error = None;
            if hwnd.is_null() || WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION) == 0
            {
                error = Some("Session event registration unavailable; desktop state is polled");
            }
            if RegisterHotKey(
                hwnd,
                0x524f,
                MOD_CONTROL | MOD_ALT | MOD_NOREPEAT,
                b'P' as u32,
            ) == 0
            {
                error = Some("Ctrl+Alt+P unavailable; use Pause Anti-AFK in the interface or tray");
            }
            Self { hwnd, error }
        }
    }
    pub(crate) fn error(&self) -> Option<&str> {
        self.error
    }
    pub(crate) fn pump(&mut self) -> bool {
        let mut toggle = false;
        unsafe {
            let mut message = std::mem::zeroed();
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if message.message == WM_HOTKEY && message.wParam == 0x524f {
                    toggle = !toggle;
                }
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        toggle
    }
}
impl Drop for Environment {
    fn drop(&mut self) {
        unsafe {
            UnregisterHotKey(self.hwnd, 0x524f);
            WTSUnRegisterSessionNotification(self.hwnd);
            DestroyWindow(self.hwnd);
        }
    }
}

fn window_owned(hwnd: HWND, pid: u32) -> bool {
    let mut owner = 0;
    unsafe {
        IsWindow(hwnd) != 0 && GetWindowThreadProcessId(hwnd, &mut owner) != 0 && owner == pid
    }
}
fn player_window(pid: u32) -> Result<HWND, String> {
    struct Search {
        pid: u32,
        windows: Vec<HWND>,
    }
    unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> i32 {
        let search = unsafe { &mut *(data as *mut Search) };
        if window_owned(hwnd, search.pid)
            && unsafe { IsWindowVisible(hwnd) != 0 && GetWindow(hwnd, GW_OWNER).is_null() }
        {
            search.windows.push(hwnd);
        }
        1
    }
    let mut search = Search {
        pid,
        windows: Vec::new(),
    };
    unsafe {
        EnumWindows(Some(visit), &mut search as *mut _ as LPARAM);
    }
    if search.windows.len() != 1 {
        return Err("A unique owned player window is unavailable".into());
    }
    Ok(search.windows[0])
}
fn window_matches(hwnd: HWND, show: i32) -> bool {
    unsafe {
        match show {
            SW_HIDE => IsWindowVisible(hwnd) == 0,
            SW_MINIMIZE | SW_SHOWMINIMIZED | SW_SHOWMINNOACTIVE => {
                IsWindowVisible(hwnd) != 0 && IsIconic(hwnd) != 0
            }
            SW_SHOWMAXIMIZED => IsWindowVisible(hwnd) != 0 && IsZoomed(hwnd) != 0,
            _ => IsWindowVisible(hwnd) != 0 && IsIconic(hwnd) == 0,
        }
    }
}
fn show_command(hwnd: HWND) -> Result<i32, String> {
    let mut placement: WINDOWPLACEMENT = unsafe { std::mem::zeroed() };
    placement.length = size_of::<WINDOWPLACEMENT>() as u32;
    if unsafe { GetWindowPlacement(hwnd, &mut placement) } == 0 {
        return Err("Window placement unavailable".into());
    }
    Ok(placement.showCmd as i32)
}

#[derive(Default)]
struct Original {
    power: Option<PROCESS_POWER_THROTTLING_STATE>,
    memory_priority: Option<u32>,
    priority: Option<u32>,
    affinity: Option<usize>,
    window: Option<(HWND, WINDOWPLACEMENT, bool)>,
}
#[derive(Clone, Copy, PartialEq)]
enum WindowPhase {
    MinimizeThenHide,
    Minimize,
    Hide,
    Restore,
}
#[derive(Clone, Copy)]
struct PendingWindow {
    hwnd: HWND,
    phase: WindowPhase,
    started: Instant,
}

struct Activity {
    process: platform::ControlProcess,
    hwnd: HWND,
    action: IdleAction,
    started: Instant,
    last_input: u32,
    focus: FocusLease,
}

fn last_desktop_input() -> Option<u32> {
    let mut info = LASTINPUTINFO {
        cbSize: size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    (unsafe { GetLastInputInfo(&mut info) } != 0).then_some(info.dwTime)
}

fn idle_desktop_input() -> Option<u32> {
    last_desktop_input().filter(|last| {
        control_policy::desktop_idle(unsafe { GetTickCount64() } as u32, *last)
            && !(1..=255).any(|key| unsafe { GetAsyncKeyState(key) } < 0)
    })
}

fn power_policy(
    process: &platform::ControlProcess,
) -> Result<PROCESS_POWER_THROTTLING_STATE, String> {
    let mut value: PROCESS_POWER_THROTTLING_STATE = unsafe { std::mem::zeroed() };
    value.Version = PROCESS_POWER_THROTTLING_CURRENT_VERSION;
    if unsafe {
        GetProcessInformation(
            process.raw(),
            ProcessPowerThrottling,
            (&mut value as *mut PROCESS_POWER_THROTTLING_STATE).cast(),
            size_of_val(&value) as u32,
        )
    } == 0
    {
        return Err("EcoQoS capture/read unavailable".into());
    }
    Ok(value)
}
fn memory_priority(process: &platform::ControlProcess) -> Result<u32, String> {
    let mut value = MEMORY_PRIORITY_INFORMATION { MemoryPriority: 0 };
    if unsafe {
        GetProcessInformation(
            process.raw(),
            ProcessMemoryPriority,
            (&mut value as *mut MEMORY_PRIORITY_INFORMATION).cast(),
            size_of_val(&value) as u32,
        )
    } == 0
    {
        return Err("Memory priority capture/read unavailable".into());
    }
    Ok(value.MemoryPriority)
}
fn set_power(process: &platform::ControlProcess, value: &PROCESS_POWER_THROTTLING_STATE) -> bool {
    unsafe {
        SetProcessInformation(
            process.raw(),
            ProcessPowerThrottling,
            (value as *const PROCESS_POWER_THROTTLING_STATE).cast(),
            size_of_val(value) as u32,
        ) != 0
    }
}
fn set_memory_priority(process: &platform::ControlProcess, value: u32) -> bool {
    let value = MEMORY_PRIORITY_INFORMATION {
        MemoryPriority: value,
    };
    unsafe {
        SetProcessInformation(
            process.raw(),
            ProcessMemoryPriority,
            (&value as *const MEMORY_PRIORITY_INFORMATION).cast(),
            size_of_val(&value) as u32,
        ) != 0
    }
}
fn same_power(a: &PROCESS_POWER_THROTTLING_STATE, b: &PROCESS_POWER_THROTTLING_STATE) -> bool {
    a.ControlMask == b.ControlMask && a.StateMask == b.StateMask
}
fn background_power(
    mut original: PROCESS_POWER_THROTTLING_STATE,
) -> PROCESS_POWER_THROTTLING_STATE {
    original.ControlMask |= PROCESS_POWER_THROTTLING_EXECUTION_SPEED;
    original.StateMask |= PROCESS_POWER_THROTTLING_EXECUTION_SPEED;
    original
}
fn background_priority(original: u32) -> u32 {
    if original == IDLE_PRIORITY_CLASS {
        original
    } else {
        BELOW_NORMAL_PRIORITY_CLASS
    }
}
fn efficiency_status(process: &platform::ControlProcess) -> String {
    let power = power_policy(process)
        .map(|p| {
            if p.ControlMask & p.StateMask & PROCESS_POWER_THROTTLING_EXECUTION_SPEED != 0 {
                "enabled"
            } else {
                "not explicitly enabled"
            }
        })
        .unwrap_or("unavailable");
    let memory = memory_priority(process)
        .map(|p| p.to_string())
        .unwrap_or_else(|_| "unavailable".into());
    format!("EcoQoS {power}; memory priority {memory}")
}

pub(crate) struct Managed {
    identity: ProcessIdentity,
    original: Option<Original>,
    applied: Option<ResourceSettings>,
    applied_client: Option<ClientSettings>,
    cpu: Option<(u64, Instant)>,
    faults: Option<(u32, Instant)>,
    last_input: String,
    last_activity_attempt_at: Option<chrono::DateTime<chrono::Utc>>,
    last_input_submitted_at: Option<chrono::DateTime<chrono::Utc>>,
    activity: Option<Activity>,
    pending_release: Vec<u16>,
    pending_window: Option<PendingWindow>,
    pending_process: Option<platform::ControlProcess>,
    window_error: Option<String>,
    warnings: Vec<String>,
    restoring: bool,
}
impl Managed {
    pub(crate) fn new(identity: ProcessIdentity) -> Self {
        Self {
            identity,
            original: None,
            applied: None,
            applied_client: None,
            cpu: None,
            faults: None,
            last_input: "No input submitted".into(),
            last_activity_attempt_at: None,
            last_input_submitted_at: None,
            activity: None,
            pending_release: Vec::new(),
            pending_window: None,
            pending_process: None,
            window_error: None,
            warnings: Vec::new(),
            restoring: false,
        }
    }
    pub(crate) fn identity(&self) -> &ProcessIdentity {
        &self.identity
    }
    pub(crate) fn activity_status(&self, result: &mut RuntimeControls) {
        result.input_result = self.last_input.clone();
        result.last_activity_attempt_at = self.last_activity_attempt_at;
        result.last_input_submitted_at = self.last_input_submitted_at;
    }
    pub(crate) fn has_activity(&self) -> bool {
        self.activity.is_some()
    }
    pub(crate) fn start_activity(
        &mut self,
        process: &platform::ControlProcess,
        action: IdleAction,
    ) -> bool {
        self.last_activity_attempt_at = Some(chrono::Utc::now());
        let result = (|| {
            if suspended()
                || self.has_pending_window()
                || self.has_activity()
                || self
                    .applied_client
                    .as_ref()
                    .is_none_or(|c| c.mode != ClientMode::Headless)
            {
                return Err("Activity deferred: session or window transition busy");
            }
            let last_input = idle_desktop_input()
                .ok_or("Activity deferred: desktop has not been idle for 60 seconds")?;
            let hwnd = self
                .original
                .as_ref()
                .and_then(|o| o.window)
                .map(|w| w.0)
                .filter(|h| window_owned(*h, self.identity.pid) && window_matches(*h, SW_HIDE))
                .ok_or("Activity deferred: verified hidden window unavailable")?;
            let process = process
                .duplicate()
                .map_err(|_| "Activity deferred: process handle unavailable")?;
            let focus = FocusLease::capture(hwnd, self.identity.pid, SW_SHOWNORMAL);
            if !focus.previous.is_null() && focus.previous_process.is_none() {
                return Err("Activity deferred: previous foreground ownership unavailable");
            }
            self.activity = Some(Activity {
                process,
                hwnd,
                action,
                started: Instant::now(),
                last_input,
                focus,
            });
            if unsafe { ShowWindowAsync(hwnd, SW_RESTORE) } == 0 {
                self.cancel_activity(true);
                return Err("Activity deferred: window restore denied");
            }
            Ok(())
        })();
        self.last_input = match result {
            Ok(()) => {
                "Experimental activity: restoring window; Roblox acceptance unverified".into()
            }
            Err(error) => error.into(),
        };
        result.is_ok()
    }
    pub(crate) fn cancel_activity(&mut self, rehide: bool) {
        if let Some(activity) = self.activity.take() {
            drop(activity.focus);
            if rehide
                && window_owned(activity.hwnd, self.identity.pid)
                && unsafe { WaitForSingleObject(activity.process.raw(), 0) } == WAIT_TIMEOUT
            {
                self.window_error = None;
                self.request_window(
                    &activity.process,
                    activity.hwnd,
                    WindowPhase::MinimizeThenHide,
                );
            }
        }
    }
    /// Returns the OS submission outcome only when an activity attempt finishes.
    pub(crate) fn advance_activity(&mut self, eligible: impl Fn() -> bool) -> Option<bool> {
        let activity = self.activity.as_ref()?;
        let ready = unsafe { IsWindowVisible(activity.hwnd) != 0 && IsIconic(activity.hwnd) == 0 };
        let valid = eligible()
            && !suspended()
            && window_owned(activity.hwnd, self.identity.pid)
            && unsafe { WaitForSingleObject(activity.process.raw(), 0) } == WAIT_TIMEOUT
            && last_desktop_input() == Some(activity.last_input);
        if valid && !ready && activity.started.elapsed() < std::time::Duration::from_secs(3) {
            return None;
        }
        let activity = self.activity.take().unwrap();
        let result = if valid && ready {
            self.send_input(activity.hwnd, activity.action, &activity.process, || {
                !eligible() || last_desktop_input() != Some(activity.last_input)
            })
        } else {
            Err("Activity cancelled: input, preferences, session or window changed; restore deadline is three seconds".into())
        };
        let submitted = result.is_ok();
        self.record_input(result);
        self.activity = Some(activity);
        self.cancel_activity(true);
        Some(submitted)
    }
    pub(crate) fn sample(
        &mut self,
        process: &platform::ControlProcess,
        result: &mut RuntimeControls,
    ) {
        unsafe {
            let mut created = std::mem::zeroed();
            let mut exited = created;
            let mut kernel = created;
            let mut user = created;
            let now = Instant::now();
            if GetProcessTimes(
                process.raw(),
                &mut created,
                &mut exited,
                &mut kernel,
                &mut user,
            ) != 0
            {
                let ticks = |time: FILETIME| {
                    ((time.dwHighDateTime as u64) << 32) | time.dwLowDateTime as u64
                };
                let current = ticks(kernel) + ticks(user);
                let now_ms = (chrono::Utc::now().timestamp_millis() + 11_644_473_600_000) as u64;
                result.process_uptime_seconds =
                    Some(now_ms.saturating_sub(ticks(created) / 10_000) / 1000);
                if let Some((previous, at)) = self.cpu {
                    result.cpu_percent = control_policy::cpu_percent(
                        previous,
                        current,
                        now.duration_since(at),
                        GetActiveProcessorCount(ALL_PROCESSOR_GROUPS) as usize,
                    );
                }
                self.cpu = Some((current, now));
            }
            if result.cpu_percent.is_none() {
                result
                    .unavailable
                    .push("CPU measurement unavailable or awaiting second sample".into());
            }
            let mut memory: PROCESS_MEMORY_COUNTERS_EX = std::mem::zeroed();
            memory.cb = size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
            if GetProcessMemoryInfo(
                process.raw(),
                (&mut memory as *mut PROCESS_MEMORY_COUNTERS_EX).cast(),
                size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
            ) != 0
            {
                result.working_set_bytes = Some(memory.WorkingSetSize as u64);
                result.private_bytes = Some(memory.PrivateUsage as u64);
                if let Some((previous, at)) = self.faults {
                    result.page_faults_per_second = control_policy::page_fault_rate(
                        previous,
                        memory.PageFaultCount,
                        now.duration_since(at),
                    );
                }
                self.faults = Some((memory.PageFaultCount, now));
            } else {
                self.faults = None;
                result
                    .unavailable
                    .push("Working set/private memory/page faults unavailable".into());
            }
        }
    }
    pub(crate) fn has_pending_window(&self) -> bool {
        self.pending_window.is_some()
    }
    pub(crate) fn matches_preferences(
        &self,
        settings: &ResourceSettings,
        client: &ClientSettings,
    ) -> bool {
        self.applied.as_ref() == Some(settings) && self.applied_client.as_ref() == Some(client)
    }
    pub(crate) fn cancel_window_transition(&mut self) {
        self.pending_window = None;
        self.pending_process = None;
    }
    pub(crate) fn advance_pending(&mut self) {
        if let Some(process) = self.pending_process.take() {
            self.advance_window(&process);
            if self.pending_window.is_some() {
                self.pending_process = Some(process);
            }
        }
    }
    pub(crate) fn advance_window(&mut self, process: &platform::ControlProcess) {
        let Some(pending) = self.pending_window else {
            return;
        };
        if !window_owned(pending.hwnd, self.identity.pid)
            || unsafe { WaitForSingleObject(process.raw(), 0) } != WAIT_TIMEOUT
        {
            self.pending_window = None;
            self.pending_process = None;
            self.window_error = Some("Window/process changed during transition".into());
            return;
        }
        let done = match pending.phase {
            WindowPhase::MinimizeThenHide if unsafe { IsIconic(pending.hwnd) } != 0 => {
                if unsafe { ShowWindowAsync(pending.hwnd, SW_HIDE) } == 0 {
                    self.window_error = Some("Window hide request denied".into());
                    self.pending_window = None;
                } else if let Some(next) = &mut self.pending_window {
                    next.phase = WindowPhase::Hide;
                }
                false
            }
            WindowPhase::MinimizeThenHide => false,
            WindowPhase::Hide => window_matches(pending.hwnd, SW_HIDE),
            WindowPhase::Minimize => window_matches(pending.hwnd, SW_MINIMIZE),
            WindowPhase::Restore => self.original_window_matches(),
        };
        if done {
            self.pending_window = None;
        } else if pending.started.elapsed() >= std::time::Duration::from_secs(10) {
            self.pending_window = None;
            self.window_error = Some("Window transition exceeded ten seconds".into());
        }
        if self.pending_window.is_none() {
            self.pending_process = None;
        }
    }
    fn original_window_matches(&self) -> bool {
        self.original
            .as_ref()
            .and_then(|o| o.window)
            .is_none_or(|(hwnd, original, visible)| {
                if !window_owned(hwnd, self.identity.pid) {
                    return true;
                }
                let mut current: WINDOWPLACEMENT = unsafe { std::mem::zeroed() };
                current.length = size_of::<WINDOWPLACEMENT>() as u32;
                window_matches(
                    hwnd,
                    if visible {
                        original.showCmd as i32
                    } else {
                        SW_HIDE
                    },
                ) && unsafe { GetWindowPlacement(hwnd, &mut current) } != 0
                    && current.rcNormalPosition.left == original.rcNormalPosition.left
                    && current.rcNormalPosition.top == original.rcNormalPosition.top
                    && current.rcNormalPosition.right == original.rcNormalPosition.right
                    && current.rcNormalPosition.bottom == original.rcNormalPosition.bottom
            })
    }
    fn request_window(
        &mut self,
        process: &platform::ControlProcess,
        hwnd: HWND,
        phase: WindowPhase,
    ) {
        let process = match process.duplicate() {
            Ok(process) => process,
            Err(error) => {
                self.window_error = Some(error);
                return;
            }
        };
        self.pending_process = Some(process);
        self.pending_window = Some(PendingWindow {
            hwnd,
            phase,
            started: Instant::now(),
        });
        let requested = unsafe {
            if phase == WindowPhase::Restore {
                let (_, mut placement, visible) = self.original.as_ref().unwrap().window.unwrap();
                placement.flags |= WPF_ASYNCWINDOWPLACEMENT;
                SetWindowPlacement(hwnd, &placement) != 0
                    && (self.original_window_matches()
                        || ShowWindowAsync(
                            hwnd,
                            if visible {
                                placement.showCmd as i32
                            } else {
                                SW_HIDE
                            },
                        ) != 0)
            } else {
                ShowWindowAsync(
                    hwnd,
                    if phase == WindowPhase::Hide {
                        SW_HIDE
                    } else {
                        SW_SHOWMINNOACTIVE
                    },
                ) != 0
            }
        };
        if !requested {
            self.cancel_window_transition();
            self.window_error = Some("Window state request denied".into());
        }
    }
    fn apply_process(
        &mut self,
        process: &platform::ControlProcess,
        settings: &ResourceSettings,
        effective: ResourceMode,
    ) {
        let efficient = effective == ResourceMode::LowResource;
        let original = self.original.get_or_insert_with(Original::default);
        if effective != ResourceMode::Normal && original.priority.is_none() {
            let value = unsafe { GetPriorityClass(process.raw()) };
            if value == 0 {
                self.warnings.push("Priority capture unavailable".into());
            } else {
                original.priority = Some(value);
            }
        }
        if efficient && original.power.is_none() {
            match power_policy(process) {
                Ok(value) => original.power = Some(value),
                Err(error) => self.warnings.push(error),
            }
        }
        if efficient && original.memory_priority.is_none() {
            match memory_priority(process) {
                Ok(value) => original.memory_priority = Some(value),
                Err(error) => self.warnings.push(error),
            }
        }
        if settings.affinity.is_some() && original.affinity.is_none() {
            let mut previous = 0;
            let mut system = 0;
            // shortcut: affinity supports one processor group; add group placement when larger machines need it.
            if unsafe { GetActiveProcessorGroupCount() } != 1
                || unsafe { GetProcessAffinityMask(process.raw(), &mut previous, &mut system) } == 0
            {
                self.warnings
                    .push("Affinity capture unavailable for this processor group".into());
            } else if settings
                .affinity_mask()
                .ok()
                .flatten()
                .is_some_and(|mask| mask & !system != 0)
            {
                self.warnings
                    .push("Affinity selects unavailable processors".into());
            } else {
                original.affinity = Some(previous);
            }
        }
        if let Some(value) = original.priority {
            let target = if effective == ResourceMode::Normal {
                value
            } else {
                background_priority(value)
            };
            if unsafe { SetPriorityClass(process.raw(), target) } == 0 {
                self.warnings.push("CPU priority request denied".into());
            }
        }
        if let Some(original) = &original.power {
            let target = if efficient {
                background_power(*original)
            } else {
                *original
            };
            if !set_power(process, &target) {
                self.warnings.push("EcoQoS request denied".into());
            }
        }
        if let Some(value) = original.memory_priority
            && !set_memory_priority(
                process,
                if efficient {
                    value.min(MEMORY_PRIORITY_LOW)
                } else {
                    value
                },
            )
        {
            self.warnings.push("Memory priority request denied".into());
        }
        if let Some(original) = original.affinity {
            let target = settings.affinity_mask().ok().flatten().unwrap_or(original);
            if unsafe { SetProcessAffinityMask(process.raw(), target) } == 0 {
                self.warnings.push("Affinity request denied".into());
            }
        }
    }
    fn policy_result(&self, process: &platform::ControlProcess) -> String {
        let Some(settings) = &self.applied else {
            return "Original settings restoration pending".into();
        };
        let Some(client) = &self.applied_client else {
            return "Client mode unavailable".into();
        };
        let effective = settings.effective_mode(client.mode);
        let mut warnings = self.warnings.clone();
        if let Some(error) = &self.window_error {
            warnings.push(error.clone());
        }
        if let Some(original) = &self.original {
            if let Some(value) = original.priority {
                let target = if effective == ResourceMode::Normal {
                    value
                } else {
                    background_priority(value)
                };
                if unsafe { GetPriorityClass(process.raw()) } != target {
                    warnings.push("CPU priority differs from request".into());
                }
            }
            if let Some(original) = &original.power {
                let target = if effective == ResourceMode::LowResource {
                    background_power(*original)
                } else {
                    *original
                };
                if !power_policy(process).is_ok_and(|p| same_power(&p, &target)) {
                    warnings.push("EcoQoS policy not verified".into());
                }
            }
            if let Some(value) = original.memory_priority {
                let target = if effective == ResourceMode::LowResource {
                    value.min(MEMORY_PRIORITY_LOW)
                } else {
                    value
                };
                if memory_priority(process) != Ok(target) {
                    warnings.push("Memory priority not verified".into());
                }
            }
            if let Some(original) = original.affinity {
                let mut current = 0;
                let mut system = 0;
                let target = settings.affinity_mask().ok().flatten().unwrap_or(original);
                if unsafe { GetProcessAffinityMask(process.raw(), &mut current, &mut system) } == 0
                    || current != target
                {
                    warnings.push("Affinity not verified".into());
                }
            }
        }
        let phase = self
            .pending_window
            .map(|p| match p.phase {
                WindowPhase::MinimizeThenHide | WindowPhase::Minimize => "minimizing requested",
                WindowPhase::Hide => "hiding requested",
                WindowPhase::Restore => "restoring requested",
            })
            .unwrap_or("window observed");
        let profile = match effective {
            ResourceMode::Normal => "Default",
            ResourceMode::Balanced => "Balanced",
            ResourceMode::LowResource => "Low Resource",
        };
        format!(
            "{profile}{}; {phase}; {}{}; rendering remains unverified",
            if client.mode == ClientMode::Headless {
                " (automatic in Headless)"
            } else {
                ""
            },
            efficiency_status(process),
            if warnings.is_empty() {
                String::new()
            } else {
                format!("; partial optimization: {}", warnings.join("; "))
            }
        )
    }
    pub(crate) fn apply(
        &mut self,
        process: &platform::ControlProcess,
        settings: &ResourceSettings,
        client: &ClientSettings,
    ) -> String {
        if let Err(error) = client.validate().and_then(|_| settings.validate()) {
            return error;
        }
        let changed = !self.matches_preferences(settings, client);
        if changed {
            self.cancel_activity(false);
        }
        if self.has_activity() {
            return "Experimental foreground activity in progress; resource policies retained"
                .into();
        }
        let effective = settings.effective_mode(client.mode);
        let normal =
            effective == ResourceMode::Normal && settings.affinity.is_none() && !client.auto_hide;
        if changed {
            self.cancel_window_transition();
            self.window_error = None;
            self.warnings.clear();
            self.restoring = false;
            self.applied = Some(settings.clone());
            self.applied_client = Some(client.clone());
        } else {
            self.advance_window(process);
        }
        if normal {
            return match self.restore_with(process) {
                Ok(()) => "Normal; original process and window settings restored".into(),
                Err(error) => format!("Restoring original settings: {error}"),
            };
        }
        let window = self.original.as_ref().and_then(|o| o.window).map(|w| w.0);
        let recreated = window.is_some_and(|hwnd| !window_owned(hwnd, self.identity.pid));
        if recreated {
            self.original.as_mut().unwrap().window = None;
            self.cancel_window_transition();
            self.window_error = None;
        }
        if changed || recreated {
            self.apply_process(process, settings, effective);
        }
        if self.pending_window.is_none() && self.window_error.is_none() {
            let phase = if client.mode == ClientMode::Headless
                || client.auto_hide && effective == ResourceMode::LowResource
            {
                Some(WindowPhase::MinimizeThenHide)
            } else if client.auto_hide {
                Some(WindowPhase::Hide)
            } else if effective == ResourceMode::LowResource {
                Some(WindowPhase::Minimize)
            } else {
                self.original
                    .as_ref()
                    .and_then(|o| o.window)
                    .map(|_| WindowPhase::Restore)
            };
            if let Some(phase) = phase {
                let original = self.original.get_or_insert_with(Original::default);
                if original.window.is_none() && phase != WindowPhase::Restore {
                    match player_window(self.identity.pid) {
                        Ok(hwnd) => {
                            let mut placement: WINDOWPLACEMENT = unsafe { std::mem::zeroed() };
                            placement.length = size_of::<WINDOWPLACEMENT>() as u32;
                            if unsafe { GetWindowPlacement(hwnd, &mut placement) } == 0 {
                                return "Window placement capture unavailable; process optimization only".into();
                            }
                            original.window =
                                Some((hwnd, placement, unsafe { IsWindowVisible(hwnd) != 0 }));
                        }
                        Err(error) => return format!("{error}; process optimization only"),
                    }
                }
                if let Some((hwnd, _, _)) = original.window {
                    let matches = match phase {
                        WindowPhase::MinimizeThenHide => {
                            window_matches(hwnd, SW_HIDE) && unsafe { IsIconic(hwnd) } != 0
                        }
                        WindowPhase::Hide => window_matches(hwnd, SW_HIDE),
                        WindowPhase::Minimize => window_matches(hwnd, SW_MINIMIZE),
                        WindowPhase::Restore => self.original_window_matches(),
                    };
                    if !matches {
                        self.request_window(process, hwnd, phase);
                    }
                }
            }
        }
        self.policy_result(process)
    }
    fn restore_with(&mut self, process: &platform::ControlProcess) -> Result<(), String> {
        if !self.restoring {
            self.cancel_window_transition();
            self.window_error = None;
            self.restoring = true;
        }
        let Some(original) = &self.original else {
            return Ok(());
        };
        let mut errors = Vec::new();
        unsafe {
            if let Some(value) = original.priority
                && (SetPriorityClass(process.raw(), value) == 0
                    || GetPriorityClass(process.raw()) != value)
            {
                errors.push("CPU priority restoration incomplete");
            }
            if let Some(mask) = original.affinity {
                let mut current = 0;
                let mut system = 0;
                if SetProcessAffinityMask(process.raw(), mask) == 0
                    || GetProcessAffinityMask(process.raw(), &mut current, &mut system) == 0
                    || current != mask
                {
                    errors.push("Affinity restoration incomplete");
                }
            }
        }
        if let Some(value) = &original.power
            && (!set_power(process, value)
                || !power_policy(process).is_ok_and(|p| same_power(&p, value)))
        {
            errors.push("EcoQoS restoration incomplete");
        }
        if let Some(value) = original.memory_priority
            && (!set_memory_priority(process, value) || memory_priority(process) != Ok(value))
        {
            errors.push("Memory priority restoration incomplete");
        }
        if self
            .pending_window
            .is_some_and(|p| p.phase != WindowPhase::Restore)
        {
            self.cancel_window_transition();
            self.window_error = None;
        }
        if !self.original_window_matches() {
            if self.pending_window.is_none() && self.window_error.is_none() {
                let hwnd = self.original.as_ref().unwrap().window.unwrap().0;
                self.request_window(process, hwnd, WindowPhase::Restore);
            }
            self.advance_window(process);
            if !self.original_window_matches() {
                errors.push("Window restoration pending or denied");
            }
        }
        if !errors.is_empty() {
            if self.window_error.is_some() {
                self.restoring = false;
            }
            return Err(format!(
                "{}{}",
                errors.join("; "),
                self.window_error
                    .as_ref()
                    .map(|e| format!("; {e}"))
                    .unwrap_or_default()
            ));
        }
        self.original = None;
        self.restoring = false;
        self.cancel_window_transition();
        self.window_error = None;
        self.warnings.clear();
        Ok(())
    }
    pub(crate) fn observed_mode(&self, result: &mut RuntimeControls) {
        let hwnd = self
            .original
            .as_ref()
            .and_then(|o| o.window.as_ref())
            .map(|w| w.0)
            .filter(|hwnd| window_owned(*hwnd, self.identity.pid))
            .or_else(|| player_window(self.identity.pid).ok());
        result.window_state = hwnd
            .map_or("unavailable", |hwnd| unsafe {
                if IsWindowVisible(hwnd) == 0 {
                    "hidden"
                } else if IsIconic(hwnd) != 0 {
                    "minimized"
                } else {
                    "visible"
                }
            })
            .into();
        if self.applied.is_some()
            && self.activity.is_none()
            && self.pending_window.is_none()
            && self.window_error.is_none()
            && let Some(client) = &self.applied_client
        {
            let background = client.mode == ClientMode::Headless || client.auto_hide;
            let state_matches = if background {
                result.window_state == "hidden"
                    && (client.mode != ClientMode::Headless
                        && self
                            .applied
                            .as_ref()
                            .is_none_or(|s| s.mode != ResourceMode::LowResource)
                        || hwnd.is_some_and(|h| unsafe { IsIconic(h) } != 0))
            } else if self
                .applied
                .as_ref()
                .is_some_and(|s| s.mode == ResourceMode::LowResource)
            {
                result.window_state == "minimized"
            } else {
                result.window_state != "unavailable"
            };
            if state_matches {
                result.client_mode = Some(client.mode);
            }
        }
    }
    pub(crate) fn restore(&mut self) -> Result<(), String> {
        self.cancel_activity(false);
        self.release_pending()?;
        if self.original.is_some() {
            let process = match platform::verify_controls(&self.identity, self.identity.generation)
            {
                Ok(process) => process,
                Err(_) if matches!(platform::alive(&self.identity), Ok(false)) => {
                    self.original = None;
                    self.cancel_window_transition();
                    self.window_error = None;
                    self.applied = None;
                    self.applied_client = None;
                    return Ok(());
                }
                Err(error) => return Err(error),
            };
            self.restore_with(&process)?;
        }
        self.applied = None;
        self.applied_client = None;
        Ok(())
    }
    pub(crate) fn release_pending(&mut self) -> Result<(), String> {
        self.pending_release.retain(|key| {
            let release = key_input(*key, true);
            unsafe { SendInput(1, &release, size_of::<INPUT>() as i32) != 1 }
        });
        if self.pending_release.is_empty() {
            Ok(())
        } else {
            Err("Windows denied pending key release; cleanup will retry".into())
        }
    }
    pub(crate) fn input(&mut self, action: IdleAction, cancelled: impl Fn() -> bool) -> String {
        self.last_activity_attempt_at = Some(chrono::Utc::now());
        let result = self.submit_input(action, cancelled);
        self.record_input(result);
        self.last_input.clone()
    }
    fn record_input(&mut self, result: Result<u32, String>) {
        self.last_input = match result {
            Ok(count) => {
                self.last_input_submitted_at = Some(chrono::Utc::now());
                format!("Windows submitted {count} input events; Roblox acceptance unverified")
            }
            Err(error) => error,
        };
    }
    fn submit_input(
        &mut self,
        action: IdleAction,
        cancelled: impl Fn() -> bool,
    ) -> Result<u32, String> {
        if cancelled() || suspended() {
            return Err("Input skipped: cancelled or session suspended".into());
        }
        self.release_pending()?;
        let started = Instant::now();
        let _process = platform::verify_controls(&self.identity, self.identity.generation)?;
        if started.elapsed() > std::time::Duration::from_secs(3) {
            return Err("Input skipped: delayed ownership verification".into());
        }
        let hwnd = player_window(self.identity.pid)?;
        self.send_input(hwnd, action, &_process, cancelled)
    }
    fn send_input(
        &mut self,
        hwnd: HWND,
        action: IdleAction,
        process: &platform::ControlProcess,
        cancelled: impl Fn() -> bool,
    ) -> Result<u32, String> {
        if cancelled() || suspended() {
            return Err("Input skipped: cancelled or session suspended".into());
        }
        self.release_pending()?;
        let keys: &[u16] = match action {
            IdleAction::Space => &[VK_SPACE],
            IdleAction::Camera => &[b'I' as u16, b'O' as u16],
        };
        if modifiers_or_keys_held(keys) {
            return Err("Input skipped: modifier or action key held".into());
        }
        let lease = FocusLease::capture(hwnd, self.identity.pid, show_command(hwnd)?);
        unsafe {
            if IsIconic(hwnd) != 0 {
                ShowWindowAsync(hwnd, SW_RESTORE);
            }
        }
        // Windows may deny foreground activation. Never bypass the foreground policy.
        if unsafe { SetForegroundWindow(hwnd) } == 0 || unsafe { GetForegroundWindow() } != hwnd {
            return Err("Input skipped: foreground focus denied".into());
        }
        if cancelled()
            || suspended()
            || !window_owned(hwnd, self.identity.pid)
            || unsafe { WaitForSingleObject(process.raw(), 0) } != WAIT_TIMEOUT
            || unsafe { GetForegroundWindow() } != hwnd
            || modifiers_or_keys_held(keys)
        {
            return Err("Input skipped: focus, keys or session changed".into());
        }
        let inputs: Vec<_> = keys
            .iter()
            .flat_map(|key| [key_input(*key, false), key_input(*key, true)])
            .collect();
        let submitted = unsafe {
            SendInput(
                inputs.len() as u32,
                inputs.as_ptr(),
                size_of::<INPUT>() as i32,
            )
        };
        // Down/up pairs are one OS submission; partial submission still requires a key release.
        if submitted as usize != inputs.len() {
            if submitted % 2 == 1 {
                self.pending_release.push(keys[(submitted / 2) as usize]);
                self.release_pending()?;
            }
            return Err(
                "Partial or denied OS input submission; Roblox acceptance unverified".into(),
            );
        }
        drop(lease);
        Ok(submitted)
    }
}
fn key_input(key: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                dwFlags: if up { KEYEVENTF_KEYUP } else { 0 },
                ..unsafe { std::mem::zeroed() }
            },
        },
    }
}
fn modifiers_or_keys_held(keys: &[u16]) -> bool {
    [VK_SHIFT, VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN]
        .iter()
        .chain(keys)
        .any(|key| unsafe { GetAsyncKeyState(*key as i32) } < 0)
}
fn process_stamp(pid: u32) -> Option<u64> {
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }
        let mut created = std::mem::zeroed();
        let mut exited = created;
        let mut kernel = created;
        let mut user = created;
        let ok = GetProcessTimes(process, &mut created, &mut exited, &mut kernel, &mut user);
        CloseHandle(process);
        (ok != 0).then_some(((created.dwHighDateTime as u64) << 32) | created.dwLowDateTime as u64)
    }
}
struct FocusLease {
    hwnd: HWND,
    pid: u32,
    show: i32,
    previous: HWND,
    previous_pid: u32,
    previous_process: Option<u64>,
}
impl FocusLease {
    fn capture(hwnd: HWND, pid: u32, show: i32) -> Self {
        let previous = unsafe { GetForegroundWindow() };
        let mut previous_pid = 0;
        unsafe {
            GetWindowThreadProcessId(previous, &mut previous_pid);
        }
        Self {
            hwnd,
            pid,
            show,
            previous,
            previous_pid,
            previous_process: process_stamp(previous_pid),
        }
    }
}
impl Drop for FocusLease {
    fn drop(&mut self) {
        unsafe {
            let ours = GetForegroundWindow() == self.hwnd;
            if self.show == SW_SHOWMINIMIZED && window_owned(self.hwnd, self.pid) {
                ShowWindowAsync(self.hwnd, SW_SHOWMINNOACTIVE);
            }
            if ours
                && self.previous != self.hwnd
                && window_owned(self.previous, self.previous_pid)
                && self.previous_process.is_some()
                && process_stamp(self.previous_pid) == self.previous_process
            {
                SetForegroundWindow(self.previous);
            }
        }
    }
}

#[cfg(test)]
#[path = "client_acceptance.rs"]
mod acceptance;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancelled_input_and_pid_reuse_are_rejected_before_window_control() {
        let identity = platform::control_fixture_identity(uuid::Uuid::new_v4());
        let mut managed = Managed::new(identity.clone());
        assert!(
            managed
                .input(IdleAction::Space, || true)
                .contains("cancelled")
        );
        assert!(managed.last_activity_attempt_at.is_some());
        assert!(managed.last_input_submitted_at.is_none());
        let mut reused = identity.clone();
        reused.creation_time = "0".into();
        assert!(platform::verify_controls(&reused, reused.generation).is_err());
        assert!(platform::verify_controls(&identity, uuid::Uuid::new_v4()).is_err());
        let down = key_input(VK_SPACE, false);
        let up = key_input(VK_SPACE, true);
        unsafe {
            assert_eq!(down.Anonymous.ki.wVk, VK_SPACE);
            assert_eq!(up.Anonymous.ki.dwFlags, KEYEVENTF_KEYUP);
        }
        managed.restore().unwrap();
    }

    #[test]
    fn background_policy_preserves_lower_priority_and_unrelated_power_bits() {
        assert_eq!(
            background_priority(IDLE_PRIORITY_CLASS),
            IDLE_PRIORITY_CLASS
        );
        assert_eq!(
            background_priority(NORMAL_PRIORITY_CLASS),
            BELOW_NORMAL_PRIORITY_CLASS
        );
        let original = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION,
            StateMask: PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION,
        };
        let background = background_power(original);
        assert_eq!(
            background.ControlMask,
            original.ControlMask | PROCESS_POWER_THROTTLING_EXECUTION_SPEED
        );
        assert_eq!(
            background.StateMask,
            original.StateMask | PROCESS_POWER_THROTTLING_EXECUTION_SPEED
        );
        assert_eq!(
            original.StateMask,
            PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION
        );
    }

    #[test]
    #[ignore = "Interactive isolated Windows fixture: run only this test with --ignored --test-threads=1"]
    fn native_fixture_minimize_restore_metrics_and_session_events() {
        let mut environment = Environment::new();
        assert!(!suspended(), "Requires an unlocked interactive desktop");
        let create_window = || unsafe {
            CreateWindowExW(
                0,
                platform::wide("STATIC").as_ptr(),
                platform::wide("RoLauncher isolated resource fixture").as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                100,
                100,
                480,
                240,
                null_mut(),
                null_mut(),
                GetModuleHandleW(null()),
                null(),
            )
        };
        let hwnd = create_window();
        assert!(!hwnd.is_null());
        struct Window(HWND);
        impl Drop for Window {
            fn drop(&mut self) {
                LOCKED.store(false, Ordering::Release);
                SLEEPING.store(false, Ordering::Release);
                unsafe {
                    DestroyWindow(self.0);
                }
            }
        }
        let mut fixture_window = Window(hwnd);
        let identity = platform::control_fixture_identity(uuid::Uuid::new_v4());
        let process = platform::verify_controls(&identity, identity.generation).unwrap();
        let priority = unsafe { GetPriorityClass(process.raw()) };
        let original_power = power_policy(&process).unwrap();
        let original_memory = memory_priority(&process).unwrap();
        struct Policies<'a>(
            &'a platform::ControlProcess,
            u32,
            PROCESS_POWER_THROTTLING_STATE,
            u32,
        );
        impl Drop for Policies<'_> {
            fn drop(&mut self) {
                unsafe {
                    SetPriorityClass(self.0.raw(), self.1);
                }
                set_power(self.0, &self.2);
                set_memory_priority(self.0, self.3);
            }
        }
        let _restore_on_failure = Policies(&process, priority, original_power, original_memory);
        let mut managed = Managed::new(identity);
        let result = managed.apply(
            &process,
            &ResourceSettings {
                mode: ResourceMode::LowResource,
                affinity: None,
            },
            &ClientSettings::default(),
        );
        assert!(result.contains("requested"), "{result}");
        let until = Instant::now() + std::time::Duration::from_secs(2);
        while unsafe { IsIconic(hwnd) } == 0 && Instant::now() < until {
            environment.pump();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_ne!(unsafe { IsIconic(hwnd) }, 0);
        assert_eq!(
            unsafe { GetPriorityClass(process.raw()) },
            BELOW_NORMAL_PRIORITY_CLASS
        );
        let mut sample = RuntimeControls::default();
        managed.sample(&process, &mut sample);
        assert!(sample.working_set_bytes.is_some());
        assert!(sample.private_bytes.is_some());
        assert!(sample.page_faults_per_second.is_none());
        std::thread::sleep(std::time::Duration::from_millis(5));
        managed.sample(&process, &mut sample);
        assert!(sample.page_faults_per_second.is_some());
        assert!(sample.process_uptime_seconds.is_some());
        assert!(same_power(
            &power_policy(&process).unwrap(),
            &background_power(original_power)
        ));
        assert_eq!(
            memory_priority(&process).unwrap(),
            original_memory.min(MEMORY_PRIORITY_LOW)
        );
        managed.restore().unwrap();
        assert_eq!(unsafe { GetPriorityClass(process.raw()) }, priority);
        assert!(same_power(
            &power_policy(&process).unwrap(),
            &original_power
        ));
        assert_eq!(memory_priority(&process).unwrap(), original_memory);
        let headless = ClientSettings {
            mode: ClientMode::Headless,
            ..Default::default()
        };
        for round in 0..2 {
            let until = Instant::now() + std::time::Duration::from_secs(2);
            loop {
                managed.apply(&process, &ResourceSettings::default(), &headless);
                environment.pump();
                managed.advance_pending();
                environment.pump();
                let mut observed = RuntimeControls::default();
                managed.observed_mode(&mut observed);
                if observed.client_mode == Some(ClientMode::Headless) {
                    assert_eq!(observed.window_state, "hidden");
                    assert!(
                        managed.pending_process.is_none(),
                        "Completed transitions must release their handle"
                    );
                    assert_ne!(
                        unsafe { IsIconic(hwnd) },
                        0,
                        "Headless must minimize before hiding"
                    );
                    assert_eq!(
                        unsafe { GetPriorityClass(process.raw()) },
                        background_priority(priority)
                    );
                    assert!(same_power(
                        &power_policy(&process).unwrap(),
                        &background_power(original_power)
                    ));
                    assert_eq!(
                        memory_priority(&process).unwrap(),
                        original_memory.min(MEMORY_PRIORITY_LOW)
                    );
                    let changed_profile = ResourceSettings {
                        mode: ResourceMode::Balanced,
                        affinity: None,
                    };
                    managed.apply(&process, &changed_profile, &headless);
                    assert_eq!(managed.original.as_ref().unwrap().priority, Some(priority));
                    assert_eq!(
                        unsafe { IsWindowVisible(hwnd) },
                        0,
                        "Changing profile must not flash the game window"
                    );
                    break;
                }
                assert!(Instant::now() < until, "Hide was not confirmed");
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            // Exercise cancellation without injecting any input into the desktop.
            managed.activity = Some(Activity {
                process: process.duplicate().unwrap(),
                hwnd,
                action: IdleAction::Space,
                started: Instant::now(),
                last_input: last_desktop_input().unwrap(),
                focus: FocusLease::capture(hwnd, managed.identity.pid, SW_SHOWNORMAL),
            });
            unsafe {
                ShowWindowAsync(hwnd, SW_RESTORE);
            }
            environment.pump();
            let mut transient = RuntimeControls::default();
            managed.observed_mode(&mut transient);
            assert!(transient.client_mode.is_none());
            assert_eq!(transient.window_state, "visible");
            if round == 0 {
                assert_eq!(managed.advance_activity(|| false), Some(false));
                assert!(managed.last_input_submitted_at.is_none());
                let until = Instant::now() + std::time::Duration::from_secs(2);
                while managed.has_pending_window() {
                    environment.pump();
                    managed.advance_pending();
                    assert!(Instant::now() < until);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                assert!(window_matches(hwnd, SW_HIDE));
            }
            let until = Instant::now() + std::time::Duration::from_secs(2);
            loop {
                managed.apply(
                    &process,
                    &ResourceSettings::default(),
                    &ClientSettings::default(),
                );
                environment.pump();
                let mut observed = RuntimeControls::default();
                managed.observed_mode(&mut observed);
                if observed.client_mode == Some(ClientMode::Normal) {
                    break;
                }
                assert!(Instant::now() < until, "Restore was not confirmed");
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            assert_ne!(unsafe { IsWindowVisible(hwnd) }, 0);
            assert!(
                !managed.has_activity(),
                "Normal request must cancel pending activity"
            );
            assert_eq!(unsafe { GetPriorityClass(process.raw()) }, priority);
            assert!(same_power(
                &power_policy(&process).unwrap(),
                &original_power
            ));
            assert_eq!(memory_priority(&process).unwrap(), original_memory);
        }
        managed.apply(&process, &ResourceSettings::default(), &headless);
        environment.pump();
        unsafe {
            DestroyWindow(hwnd);
        }
        fixture_window.0 = null_mut();
        let hwnd = create_window();
        assert!(!hwnd.is_null());
        fixture_window.0 = hwnd;
        let until = Instant::now() + std::time::Duration::from_secs(2);
        loop {
            managed.apply(&process, &ResourceSettings::default(), &headless);
            environment.pump();
            let mut observed = RuntimeControls::default();
            managed.observed_mode(&mut observed);
            if observed.client_mode == Some(ClientMode::Headless) {
                break;
            }
            assert!(
                Instant::now() < until,
                "Recreated window was not minimized and hidden"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        managed.restore().unwrap();
        environment.pump();
        assert_ne!(unsafe { IsWindowVisible(hwnd) }, 0);
        let query_only = platform::control_fixture_query_only();
        let mut denied = Managed::new(managed.identity.clone());
        let until = Instant::now() + std::time::Duration::from_secs(2);
        let mut denied_result;
        loop {
            denied_result = denied.apply(&query_only, &ResourceSettings::default(), &headless);
            environment.pump();
            let mut observed = RuntimeControls::default();
            denied.observed_mode(&mut observed);
            if observed.client_mode == Some(ClientMode::Headless) {
                break;
            }
            assert!(
                Instant::now() < until,
                "Partial optimization should still hide the window"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(
            denied_result.contains("partial optimization"),
            "{denied_result}"
        );
        assert!(denied_result.contains("request denied"), "{denied_result}");
        assert_eq!(unsafe { GetPriorityClass(process.raw()) }, priority);
        assert!(same_power(
            &power_policy(&process).unwrap(),
            &original_power
        ));
        assert!(
            denied.restore_with(&query_only).is_err(),
            "Denied restoration must retain the originals"
        );
        assert!(denied.original.is_some());
        denied.restore().unwrap();
        environment.pump();

        managed.apply(&process, &ResourceSettings::default(), &headless);
        environment.pump();
        managed.pending_window = Some(PendingWindow {
            hwnd,
            phase: WindowPhase::Hide,
            started: Instant::now() - std::time::Duration::from_secs(11),
        });
        // The fixture is visible/minimized here; no hide request has been issued yet.
        managed.advance_window(&process);
        assert!(
            managed
                .window_error
                .as_ref()
                .is_some_and(|e| e.contains("ten seconds"))
        );
        assert!(!managed.has_pending_window());
        let mut observed = RuntimeControls::default();
        managed.observed_mode(&mut observed);
        assert!(observed.client_mode.is_none());
        managed.restore().unwrap();
        environment.pump();
        assert_ne!(unsafe { IsWindowVisible(hwnd) }, 0);
        assert!(same_power(
            &power_policy(&process).unwrap(),
            &original_power
        ));
        assert_eq!(memory_priority(&process).unwrap(), original_memory);

        unsafe {
            environment_proc(hwnd, WM_WTSSESSION_CHANGE, WTS_SESSION_LOCK as usize, 0);
        }
        assert!(suspended());
        unsafe {
            environment_proc(hwnd, WM_WTSSESSION_CHANGE, WTS_SESSION_UNLOCK as usize, 0);
        }
        assert!(!suspended());
        unsafe {
            environment_proc(hwnd, WM_POWERBROADCAST, PBT_APMSUSPEND as usize, 0);
        }
        assert!(suspended());
        unsafe {
            environment_proc(hwnd, WM_POWERBROADCAST, PBT_APMRESUMEAUTOMATIC as usize, 0);
        }
        assert!(!suspended());
    }
}
