use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClientMode {
    #[default]
    Normal,
    Headless,
    NetworkOnly,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Bootstrapper {
    /// Preserves the existing installed protocol handler for migrated accounts.
    #[default]
    System,
    Official,
    Bloxstrap,
    Fishstrap,
    Custom,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ClientSettings {
    pub mode: ClientMode,
    pub bootstrapper: Bootstrapper,
    pub executable: Option<String>,
    pub auto_hide: bool,
    pub start_on_open: bool,
    pub monitoring: bool,
}
impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            mode: ClientMode::Normal,
            bootstrapper: Bootstrapper::System,
            executable: None,
            auto_hide: false,
            start_on_open: false,
            monitoring: true,
        }
    }
}
pub const NETWORK_ONLY_REASON: &str = "Unsupported: no documented standalone Roblox player-session SDK or network protocol; Open Cloud and OAuth do not implement a game client";

impl ClientSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.mode == ClientMode::NetworkOnly {
            return Err(NETWORK_ONLY_REASON.into());
        }
        if let Some(path) = &self.executable
            && (path.is_empty() || path.len() > 32760 || path.chars().any(char::is_control))
        {
            return Err("Bootstrapper path must be a valid absolute executable path".into());
        }
        if self.bootstrapper == Bootstrapper::Custom && self.executable.is_none() {
            return Err("Custom bootstrapper requires an explicit executable path".into());
        }
        if self.bootstrapper == Bootstrapper::System && self.executable.is_some() {
            return Err("Choose an explicit provider to use an executable override".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migration_is_inert_and_network_only_never_falls_back() {
        let old: ClientSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(old, ClientSettings::default());
        assert!(old.validate().is_ok());
        for mode in [
            ClientMode::Normal,
            ClientMode::Headless,
            ClientMode::NetworkOnly,
        ] {
            let settings = ClientSettings {
                mode,
                ..old.clone()
            };
            assert_eq!(settings.validate().is_ok(), mode != ClientMode::NetworkOnly);
        }
        assert!(
            ClientSettings {
                bootstrapper: Bootstrapper::Custom,
                ..old
            }
            .validate()
            .is_err()
        );
    }
}
