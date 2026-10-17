use serde::{Deserialize, Serialize};

pub const ANTI_AFK_SECONDS: u64 = 600;
pub const NO_RENDER_REASON: &str = "Unavailable: no supported rendering-off control has passed independent Roblox render, progress and connection validation";

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceMode {
    #[default]
    Normal,
    Balanced,
    LowResource,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ResourceSettings {
    pub mode: ResourceMode,
    /// Hex text avoids precision loss in JSON clients.
    pub affinity: Option<String>,
}
impl ResourceSettings {
    pub(crate) fn effective_mode(&self, client: crate::client::ClientMode) -> ResourceMode {
        if client == crate::client::ClientMode::Headless {
            ResourceMode::LowResource
        } else {
            self.mode
        }
    }
    pub fn affinity_mask(&self) -> Result<Option<usize>, String> {
        self.affinity
            .as_ref()
            .map(|text| {
                let digits = text.strip_prefix("0x").unwrap_or(text);
                if digits.is_empty()
                    || digits.len() > 16
                    || !digits.bytes().all(|c| c.is_ascii_hexdigit())
                {
                    return Err(
                        "Affinity must be a nonzero hexadecimal mask of at most 16 digits".into(),
                    );
                }
                usize::from_str_radix(digits, 16)
                    .ok()
                    .filter(|mask| *mask != 0)
                    .ok_or_else(|| {
                        "Affinity must be a nonzero mask supported by this system".into()
                    })
            })
            .transpose()
    }
    pub fn validate(&self) -> Result<(), String> {
        self.affinity_mask().map(|_| ())
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IdleAction {
    #[default]
    Space,
    Camera,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct AntiAfkSettings {
    pub enabled: bool,
    pub action: IdleAction,
    pub headless_foreground_activity: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct RuntimeControls {
    pub client_mode: Option<crate::client::ClientMode>,
    pub window_state: String,
    pub process_uptime_seconds: Option<u64>,
    pub identity: Option<crate::model::ProcessIdentity>,
    pub sampled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub cpu_percent: Option<f64>,
    pub working_set_bytes: Option<u64>,
    pub private_bytes: Option<u64>,
    pub page_faults_per_second: Option<f64>,
    pub gpu_percent: Option<f64>,
    pub gpu_engine: Option<String>,
    pub unavailable: Vec<String>,
    pub resource_result: String,
    pub input_result: String,
    pub last_activity_attempt_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_input_submitted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub roblox_acceptance: &'static str,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct AggregateMetrics {
    pub active_accounts: usize,
    pub connected_clients: usize,
    pub cpu_percent: Option<f64>,
    pub working_set_bytes: Option<u64>,
    pub measured_clients: usize,
    pub gpu_percent: Option<f64>,
}
impl AggregateMetrics {
    pub fn from_accounts(
        accounts: &[crate::model::Account],
        runtime: &std::collections::BTreeMap<String, RuntimeControls>,
    ) -> Self {
        let mut aggregate = Self::default();
        let now = chrono::Utc::now();
        for account in accounts {
            aggregate.active_accounts += usize::from(account.process.is_some());
            aggregate.connected_clients += usize::from(
                account.process.is_some() && account.status == crate::model::Status::Running,
            );
            if let Some(sample) = runtime.get(&account.id).filter(|s| {
                account.process.is_some()
                    && s.identity == account.process
                    && s.sampled_at
                        .is_some_and(|at| (0..=6).contains(&(now - at).num_seconds()))
            }) {
                if let Some(cpu) = sample.cpu_percent {
                    *aggregate.cpu_percent.get_or_insert(0.0) += cpu;
                }
                if let Some(ram) = sample.working_set_bytes {
                    *aggregate.working_set_bytes.get_or_insert(0) += ram;
                    aggregate.measured_clients += 1;
                }
            }
        }
        // Busiest per-process engines may overlap; summing them is not total device utilization.
        aggregate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn headless_efficiency_does_not_change_saved_preferences() {
        for mode in [
            ResourceMode::Normal,
            ResourceMode::Balanced,
            ResourceMode::LowResource,
        ] {
            let settings = ResourceSettings {
                mode,
                affinity: Some("0x3".into()),
            };
            let saved = serde_json::to_string(&settings).unwrap();
            assert_eq!(
                settings.effective_mode(crate::client::ClientMode::Normal),
                mode
            );
            assert_eq!(
                settings.effective_mode(crate::client::ClientMode::Headless),
                ResourceMode::LowResource
            );
            assert_eq!(serde_json::to_string(&settings).unwrap(), saved);
        }
    }
    #[test]
    fn aggregates_reject_stale_and_unowned_measurements() {
        let mut account = crate::model::Account::new("1".into(), "fixture".into());
        account.status = crate::model::Status::Running;
        account.process = Some(crate::model::ProcessIdentity {
            pid: 1,
            creation_time: "1".into(),
            tracker: "123456".into(),
            generation: account.generation,
        });
        let mut samples = std::collections::BTreeMap::from([(
            "1".into(),
            RuntimeControls {
                identity: account.process.clone(),
                sampled_at: Some(chrono::Utc::now()),
                cpu_percent: Some(0.0),
                working_set_bytes: Some(100),
                ..Default::default()
            },
        )]);
        let aggregate = AggregateMetrics::from_accounts(&[account.clone()], &samples);
        assert_eq!(aggregate.cpu_percent, Some(0.0));
        assert_eq!(aggregate.working_set_bytes, Some(100));
        assert_eq!(aggregate.connected_clients, 1);
        assert!(aggregate.gpu_percent.is_none());
        samples.get_mut("1").unwrap().sampled_at =
            Some(chrono::Utc::now() - chrono::Duration::seconds(10));
        assert!(
            AggregateMetrics::from_accounts(&[account.clone()], &samples)
                .cpu_percent
                .is_none()
        );
        samples.get_mut("1").unwrap().sampled_at = Some(chrono::Utc::now());
        samples
            .get_mut("1")
            .unwrap()
            .identity
            .as_mut()
            .unwrap()
            .creation_time = "2".into();
        assert_eq!(
            AggregateMetrics::from_accounts(&[account], &samples).measured_clients,
            0
        );
    }
    #[test]
    fn defaults_and_invalid_settings() {
        let migrated: AntiAfkSettings =
            serde_json::from_str(r#"{"enabled":true,"action":"camera"}"#).unwrap();
        assert!(migrated.enabled);
        assert!(!migrated.headless_foreground_activity);
        let defaults: ResourceSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(defaults.mode, ResourceMode::Normal);
        assert_eq!(defaults.affinity_mask().unwrap(), None);
        assert!(
            !serde_json::from_str::<AntiAfkSettings>("{}")
                .unwrap()
                .enabled
        );
        for invalid in ["0", "0x", "-1", "1_0", "10000000000000000", "xyz"] {
            assert!(
                ResourceSettings {
                    affinity: Some(invalid.into()),
                    ..defaults.clone()
                }
                .validate()
                .is_err()
            );
        }
        let settings = ResourceSettings {
            mode: ResourceMode::LowResource,
            affinity: Some("0x8000000000000001".into()),
        };
        let copy: ResourceSettings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(copy, settings);
        assert_eq!(copy.affinity_mask().unwrap(), Some(0x8000000000000001));
        assert!(serde_json::from_str::<ResourceSettings>(r#"{"mode":"hidden"}"#).is_err());
        assert!(serde_json::from_str::<AntiAfkSettings>(r#"{"action":"click"}"#).is_err());
    }
}
