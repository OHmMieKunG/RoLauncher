use std::time::{Duration, Instant};

/// Resets when disabled/paused/suspended; elapsed deadlines never accumulate.
#[derive(Default)]
pub struct Deadline(Option<Instant>);
impl Deadline {
    pub fn reschedule(&mut self, now: Instant, interval: Duration) {
        self.0 = Some(now + interval);
    }
    pub fn due(&mut self, now: Instant, eligible: bool, interval: Duration) -> bool {
        if !eligible {
            self.0 = None;
            return false;
        }
        let due = self.0.is_some_and(|deadline| now >= deadline);
        if self.0.is_none() || due {
            self.0 = Some(now + interval);
        }
        due
    }
}

pub fn page_fault_rate(previous: u32, current: u32, elapsed: Duration) -> Option<f64> {
    (!elapsed.is_zero())
        .then(|| {
            current
                .checked_sub(previous)
                .map(|n| n as f64 / elapsed.as_secs_f64())
        })
        .flatten()
}

pub fn desktop_idle(now: u32, last_input: u32) -> bool {
    // Reject implausible/backwards timestamps, while allowing the 32-bit tick wrap.
    (60_000..=i32::MAX as u32).contains(&now.wrapping_sub(last_input))
}

pub fn cpu_percent(
    previous: u64,
    current: u64,
    elapsed: Duration,
    processors: usize,
) -> Option<f64> {
    if current < previous || elapsed.is_zero() || processors == 0 {
        return None;
    }
    Some(
        ((current - previous) as f64 / 10_000_000.0 / elapsed.as_secs_f64() / processors as f64
            * 100.0)
            .clamp(0.0, 100.0),
    )
}

pub fn player_image(path: &str, local_app_data: &str) -> bool {
    let path = path.to_ascii_lowercase();
    // Keep upstream's Roblox discovery contract; Fishstrap is restricted to its installed root.
    if path.ends_with("\\robloxplayerbeta.exe") && path.contains("\\roblox\\versions\\") {
        return true;
    }
    let root = local_app_data.trim_end_matches('\\').to_ascii_lowercase();
    let prefix = format!("{root}\\fishstrap\\versions\\");
    path.strip_prefix(&prefix)
        .and_then(|rest| rest.split_once('\\'))
        .is_some_and(|(version, executable)| {
            version.starts_with("version-")
                && version.len() > 8
                && executable == "robloxplayerbeta.exe"
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn busy_desktop_defers_without_restarting_the_activity_interval() {
        let now = Instant::now();
        let interval = Duration::from_secs(600);
        let mut deadline = Deadline::default();
        assert!(!deadline.due(now, true, interval));
        assert!(deadline.due(now + interval, true, interval));
        deadline.reschedule(now + interval, Duration::from_secs(60));
        assert!(!deadline.due(now + Duration::from_secs(659), true, interval));
        assert!(deadline.due(now + Duration::from_secs(660), true, interval));
        assert!(!deadline.due(now + Duration::from_secs(661), false, interval));
        assert!(!deadline.due(now + Duration::from_secs(1000), true, interval));
    }
    #[test]
    fn idle_ticks_and_fault_rates_reject_invalid_intervals() {
        assert!(desktop_idle(100_000, 40_000));
        assert!(!desktop_idle(100_000, 40_001));
        assert!(!desktop_idle(100_000, 100_001));
        assert!(desktop_idle(30_000, u32::MAX - 30_000));
        assert_eq!(page_fault_rate(10, 20, Duration::from_secs(2)), Some(5.0));
        assert_eq!(page_fault_rate(20, 10, Duration::from_secs(2)), None);
        assert_eq!(page_fault_rate(10, 20, Duration::ZERO), None);
    }
    #[test]
    fn schedules_reset_without_catch_up() {
        let now = Instant::now();
        let interval = Duration::from_secs(600);
        let mut deadline = Deadline::default();
        assert!(!deadline.due(now, true, interval));
        assert!(!deadline.due(now + Duration::from_secs(599), true, interval));
        assert!(deadline.due(now + interval, true, interval));
        assert!(deadline.due(now + interval * 10, true, interval));
        assert!(!deadline.due(now + interval * 10, true, interval));
        assert!(!deadline.due(now + interval * 11, false, interval));
        assert!(!deadline.due(now + interval * 20, true, interval));
    }
    #[test]
    fn normalized_cpu_and_unavailable_values() {
        assert_eq!(
            cpu_percent(0, 20_000_000, Duration::from_secs(2), 4),
            Some(25.0)
        );
        assert_eq!(cpu_percent(20, 0, Duration::from_secs(2), 4), None);
        assert_eq!(cpu_percent(0, 1, Duration::ZERO, 4), None);
        assert_eq!(cpu_percent(0, 0, Duration::from_secs(2), 4), Some(0.0));
    }
    #[test]
    fn installed_roots_only() {
        assert!(player_image(
            r"D:\Roblox\Versions\version-123\RobloxPlayerBeta.exe",
            r"C:\Users\test\AppData\Local"
        ));
        for name in ["Roblox", "Fishstrap"] {
            assert!(player_image(
                &format!(
                    r"C:\Users\test\AppData\Local\{name}\Versions\version-123\RobloxPlayerBeta.exe"
                ),
                r"C:\Users\test\AppData\Local"
            ));
        }
        for path in [
            r"D:\Fishstrap\Versions\version-123\RobloxPlayerBeta.exe",
            r"C:\Users\test\AppData\Local\Fishstrap\Versions\version-123\Other.exe",
            r"C:\Users\test\AppData\Local\Fishstrap\Versions\..\RobloxPlayerBeta.exe",
        ] {
            assert!(!player_image(path, r"C:\Users\test\AppData\Local"));
        }
    }
}
