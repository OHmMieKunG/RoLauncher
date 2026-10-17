//! Providers consume the existing authenticated URI; diagnostics never include it.
use crate::client::{Bootstrapper, ClientSettings};
#[cfg(any(windows, test))]
use std::path::Path;
use std::path::PathBuf;

#[derive(Clone, Debug, serde::Serialize)]
pub struct Installation {
    pub provider: Bootstrapper,
    pub executable: Option<PathBuf>,
    pub configuration: Option<PathBuf>,
    pub available: bool,
    pub reason: String,
}

pub trait BootstrapperProvider {
    fn installation(&self, executable: Option<&str>) -> Result<Installation, String>;
    fn request(&self, settings: &ClientSettings) -> Result<LaunchRequest, String>;
}

pub enum LaunchRequest {
    System,
    Executable(PathBuf),
}
impl LaunchRequest {
    pub fn launch(&self, uri: &str, account_id: &str) -> Result<(), String> {
        match self {
            Self::System => crate::platform::launch_for(uri, account_id),
            Self::Executable(path) => {
                let mut command = std::process::Command::new(path);
                // The URI is one argument, never shell text or a diagnostic.
                command
                    .arg(uri)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null());
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    command.creation_flags(0x08000000);
                }
                command
                    .spawn()
                    .map(|_| ())
                    .map_err(|_| "Selected bootstrapper could not be started".into())
            }
        }
    }
}

#[cfg(any(windows, test))]
fn executable_from_command(command: &str) -> Option<PathBuf> {
    let command = command.trim();
    let exe = if let Some(quoted) = command.strip_prefix('"') {
        quoted.split_once('"')?.0
    } else {
        command.split_whitespace().next()?
    };
    exe.to_ascii_lowercase()
        .ends_with(".exe")
        .then(|| PathBuf::from(exe))
}

#[cfg(windows)]
fn registry_string(key: &str, value: &str) -> Option<String> {
    use windows_sys::Win32::{Foundation::ERROR_SUCCESS, System::Registry::*};
    let (hive, subkey) = key.split_once('\\')?;
    let root = match hive {
        "HKCR" => HKEY_CLASSES_ROOT,
        "HKCU" => HKEY_CURRENT_USER,
        "HKLM" => HKEY_LOCAL_MACHINE,
        _ => return None,
    };
    let subkey = crate::platform::wide(subkey);
    let value = crate::platform::wide(value);
    let mut data = vec![0u16; 32768];
    let mut bytes = (data.len() * 2) as u32;
    let result = unsafe {
        RegGetValueW(
            root,
            subkey.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ,
            std::ptr::null_mut(),
            data.as_mut_ptr().cast(),
            &mut bytes,
        )
    };
    if result != ERROR_SUCCESS {
        return None;
    }
    let length = data.iter().position(|c| *c == 0)?;
    String::from_utf16(&data[..length]).ok()
}

#[cfg(windows)]
fn registered_executable(protocol: &str) -> Option<PathBuf> {
    executable_from_command(&registry_string(
        &format!(r"HKCR\{protocol}\shell\open\command"),
        "",
    )?)
}

#[cfg(windows)]
pub(crate) fn installed_root(provider: Bootstrapper) -> Option<PathBuf> {
    let name = provider_name(provider)?;
    let roots = [
        r"HKCU\Software",
        r"HKLM\Software",
        r"HKLM\Software\WOW6432Node",
    ]
    .into_iter()
    .filter_map(|base| {
        let key = format!(r"{base}\Microsoft\Windows\CurrentVersion\Uninstall\{name}");
        registry_string(&key, "InstallLocation").map(PathBuf::from)
    });
    let registered = ["roblox-player", "roblox"]
        .into_iter()
        .filter_map(registered_executable);
    let candidates = provider_candidates(
        provider,
        registered,
        roots,
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
    );
    let path = std::fs::canonicalize(detected_executable(provider, candidates)?).ok()?;
    let text = path.to_string_lossy();
    // QueryFullProcessImageName uses Win32 paths; canonicalize uses extended paths.
    let normal = if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        text.strip_prefix(r"\\?\").unwrap_or(&text).to_owned()
    };
    Path::new(&normal).parent().map(Path::to_path_buf)
}

#[cfg(any(windows, test))]
fn provider_name(provider: Bootstrapper) -> Option<&'static str> {
    match provider {
        Bootstrapper::Bloxstrap => Some("Bloxstrap"),
        Bootstrapper::Fishstrap => Some("Fishstrap"),
        _ => None,
    }
}

#[cfg(any(windows, test))]
fn provider_candidates(
    provider: Bootstrapper,
    registered: impl IntoIterator<Item = PathBuf>,
    roots: impl IntoIterator<Item = PathBuf>,
    local: Option<PathBuf>,
) -> Vec<PathBuf> {
    let Some(name) = provider_name(provider) else {
        return Vec::new();
    };
    let executable = format!("{name}.exe");
    registered
        .into_iter()
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.eq_ignore_ascii_case(&executable))
        })
        .chain(roots.into_iter().map(|p| p.join(&executable)))
        .chain(local.map(|p| p.join(name).join(&executable)))
        .collect()
}

#[cfg(any(windows, test))]
fn detected_executable(
    provider: Bootstrapper,
    candidates: impl IntoIterator<Item = PathBuf>,
) -> Option<PathBuf> {
    candidates
        .into_iter()
        .find(|path| verify_executable(path, provider).is_ok())
}

#[cfg(any(windows, test))]
fn verify_executable(path: &Path, provider: Bootstrapper) -> Result<PathBuf, String> {
    if !path.is_absolute()
        || path
            .extension()
            .is_none_or(|e| !e.eq_ignore_ascii_case("exe"))
    {
        return Err("Choose an absolute .exe path, without arguments".into());
    }
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let expected = match provider {
        Bootstrapper::Official => Some("RobloxPlayerBeta.exe"),
        Bootstrapper::Bloxstrap => Some("Bloxstrap.exe"),
        Bootstrapper::Fishstrap => Some("Fishstrap.exe"),
        _ => None,
    };
    if expected.is_some_and(|expected| !name.eq_ignore_ascii_case(expected)) {
        return Err("Executable name does not match the selected provider".into());
    }
    use std::io::{Read, Seek, SeekFrom};
    let mut file =
        std::fs::File::open(path).map_err(|_| "Bootstrapper executable is unavailable")?;
    let mut header = [0; 64];
    file.read_exact(&mut header)
        .map_err(|_| "Invalid Windows executable")?;
    if &header[..2] != b"MZ" {
        return Err("Invalid Windows executable".into());
    }
    let offset = u32::from_le_bytes(header[60..64].try_into().unwrap());
    file.seek(SeekFrom::Start(offset as u64))
        .map_err(|_| "Invalid PE header")?;
    let mut signature = [0; 4];
    file.read_exact(&mut signature)
        .map_err(|_| "Invalid PE header")?;
    if &signature != b"PE\0\0" {
        return Err("Invalid PE header".into());
    }
    std::fs::canonicalize(path).map_err(|_| "Bootstrapper path cannot be resolved".into())
}

impl BootstrapperProvider for Bootstrapper {
    fn installation(&self, executable: Option<&str>) -> Result<Installation, String> {
        #[cfg(not(windows))]
        {
            let _ = executable;
            if *self != Bootstrapper::System {
                return Err("Windows bootstrappers unavailable on this platform".into());
            }
            Ok(Installation {
                provider: *self,
                executable: None,
                configuration: None,
                available: false,
                reason: "Windows bootstrappers unavailable on this platform".into(),
            })
        }
        #[cfg(windows)]
        {
            let registered = registered_executable("roblox-player");
            let candidate = if let Some(path) = executable {
                PathBuf::from(path)
            } else {
                match self {
                    Bootstrapper::System => registered
                        .clone()
                        .ok_or("Roblox protocol handler unavailable")?,
                    Bootstrapper::Custom => {
                        return Err("Custom provider requires an executable".into());
                    }
                    Bootstrapper::Bloxstrap | Bootstrapper::Fishstrap => {
                        let name = if *self == Bootstrapper::Bloxstrap {
                            "Bloxstrap.exe"
                        } else {
                            "Fishstrap.exe"
                        };
                        installed_root(*self).map(|p| p.join(name)).ok_or(
                            "Provider installation not found; choose its installed executable",
                        )?
                    }
                    Bootstrapper::Official => {
                        if let Some(path) = registered.filter(|p| {
                            p.file_name().is_some_and(|name| {
                                name.eq_ignore_ascii_case("RobloxPlayerBeta.exe")
                            })
                        }) {
                            path
                        } else {
                            let local = std::env::var_os("LOCALAPPDATA")
                                .ok_or("LOCALAPPDATA unavailable")?;
                            let mut candidates: Vec<_> = std::fs::read_dir(PathBuf::from(local).join("Roblox/Versions"))
                                .map_err(|_| "Official Roblox installation unavailable; choose its installed executable")?
                                .filter_map(Result::ok).map(|e| e.path().join("RobloxPlayerBeta.exe"))
                                .filter(|p| p.is_file()).collect();
                            candidates.sort_by_key(|p| {
                                std::cmp::Reverse(p.metadata().and_then(|m| m.modified()).ok())
                            });
                            candidates
                                .into_iter()
                                .next()
                                .ok_or("Official Roblox executable unavailable")?
                        }
                    }
                }
            };
            let path = verify_executable(&candidate, *self)?;
            let mut available = true;
            let mut reason =
                "Executable verified; player ownership and connection verified after launch"
                    .to_owned();
            if matches!(self, Bootstrapper::Bloxstrap | Bootstrapper::Fishstrap) {
                let player = registered_executable("roblox-player")
                    .and_then(|p| std::fs::canonicalize(p).ok());
                let deeplink =
                    registered_executable("roblox").and_then(|p| std::fs::canonicalize(p).ok());
                if !protocols_match(&path, player.as_deref(), deeplink.as_deref()) {
                    available = false;
                    reason = "Detected, but not registered for both roblox-player and roblox. Select it in that launcher's settings before use; competing Bloxstrap/Fishstrap registration is unsupported".into();
                }
            }
            let configuration = matches!(self, Bootstrapper::Bloxstrap | Bootstrapper::Fishstrap)
                .then(|| path.parent().unwrap().join("Settings.json"))
                .filter(|p| p.is_file());
            Ok(Installation {
                provider: *self,
                executable: Some(path),
                configuration,
                available,
                reason,
            })
        }
    }
    fn request(&self, settings: &ClientSettings) -> Result<LaunchRequest, String> {
        settings.validate()?;
        if *self == Bootstrapper::System {
            return Ok(LaunchRequest::System);
        }
        let installation = self.installation(settings.executable.as_deref())?;
        if !installation.available {
            return Err(installation.reason);
        }
        Ok(LaunchRequest::Executable(
            installation
                .executable
                .ok_or("Provider executable unavailable")?,
        ))
    }
}

#[cfg(any(windows, test))]
fn protocols_match(selected: &Path, player: Option<&Path>, deeplink: Option<&Path>) -> bool {
    player == Some(selected) && deeplink == Some(selected)
}

pub fn installations() -> Vec<Installation> {
    [
        Bootstrapper::System,
        Bootstrapper::Official,
        Bootstrapper::Bloxstrap,
        Bootstrapper::Fishstrap,
    ]
    .into_iter()
    .map(|provider| {
        provider
            .installation(None)
            .unwrap_or_else(|reason| Installation {
                provider,
                executable: None,
                configuration: None,
                available: false,
                reason,
            })
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    #[ignore = "Read-only installed-launcher inspection; no Roblox launch or registry writes"]
    fn host_installed_launchers_read_only() {
        let providers = installations();
        for p in &providers {
            println!(
                "{}",
                serde_json::json!({"provider":p.provider,"detected":p.executable.is_some(),
                "available":p.available,"executable_name":p.executable.as_ref().and_then(|p|p.file_name()).map(|n|n.to_string_lossy())})
            );
        }
        if let Ok(expected) = std::env::var("ROLAUNCHER_EXPECT_BOOTSTRAPPER") {
            assert!(
                providers.iter().any(|p| format!("{:?}", p.provider)
                    .eq_ignore_ascii_case(&expected)
                    && p.available
                    && p.executable.is_some()),
                "Expected host provider was not detected as available"
            );
        }
    }
    #[test]
    fn auto_detection_uses_registered_then_registry_then_default_and_skips_stale_paths() {
        let base = std::env::current_dir().unwrap().join("target");
        let root = base.join(format!("bootstrapper-detection-{}", uuid::Uuid::new_v4()));
        let local = root.join("local");
        let fish = local.join("Fishstrap/Fishstrap.exe");
        std::fs::create_dir_all(fish.parent().unwrap()).unwrap();
        // Signature-only fixture; never executed.
        let mut pe = vec![0; 68];
        pe[..2].copy_from_slice(b"MZ");
        pe[60..64].copy_from_slice(&64u32.to_le_bytes());
        pe[64..].copy_from_slice(b"PE\0\0");
        std::fs::write(&fish, &pe).unwrap();
        let registered = root.join("portable/Fishstrap.exe");
        let candidates = provider_candidates(
            Bootstrapper::Fishstrap,
            [root.join("Bloxstrap.exe"), registered.clone()],
            [root.join("stale")],
            Some(local.clone()),
        );
        assert_eq!(
            candidates,
            vec![
                registered.clone(),
                root.join("stale/Fishstrap.exe"),
                fish.clone()
            ]
        );
        assert_eq!(
            detected_executable(Bootstrapper::Fishstrap, candidates.clone()),
            Some(fish)
        );
        std::fs::create_dir_all(registered.parent().unwrap()).unwrap();
        std::fs::write(&registered, b"invalid executable").unwrap();
        assert!(detected_executable(Bootstrapper::Fishstrap, candidates.clone()).is_some());
        std::fs::write(&registered, pe).unwrap();
        assert_eq!(
            detected_executable(Bootstrapper::Fishstrap, candidates),
            Some(registered)
        );
        assert!(provider_candidates(Bootstrapper::Custom, [], [], Some(local)).is_empty());
        let resolved = std::fs::canonicalize(&root).unwrap();
        assert!(resolved.starts_with(std::fs::canonicalize(base).unwrap()));
        std::fs::remove_dir_all(resolved).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn detected_but_unregistered_provider_is_visible_and_cannot_launch() {
        let base = std::env::current_dir().unwrap().join("target");
        let root = base.join(format!("bootstrapper-guard-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("Bloxstrap.exe");
        std::fs::copy(std::env::current_exe().unwrap(), &path).unwrap();
        let path = path.to_str().unwrap();
        let installation = Bootstrapper::Bloxstrap.installation(Some(path)).unwrap();
        assert!(installation.executable.is_some());
        assert!(!installation.available);
        assert!(
            Bootstrapper::Bloxstrap
                .request(&ClientSettings {
                    bootstrapper: Bootstrapper::Bloxstrap,
                    executable: Some(path.into()),
                    ..Default::default()
                })
                .is_err()
        );
        let resolved = std::fs::canonicalize(&root).unwrap();
        assert!(resolved.starts_with(std::fs::canonicalize(base).unwrap()));
        std::fs::remove_dir_all(resolved).unwrap();
    }
    #[test]
    fn commands_are_parsed_without_running_a_shell() {
        assert_eq!(
            executable_from_command(r#""C:\Program Files\Fishstrap\Fishstrap.exe" -player "%1""#),
            Some(PathBuf::from(r"C:\Program Files\Fishstrap\Fishstrap.exe"))
        );
        assert!(executable_from_command("cmd /c evil.exe").is_none());
        assert!(executable_from_command(r#""unterminated.exe"#).is_none());
        let fish = Path::new("Fishstrap.exe");
        let blox = Path::new("Bloxstrap.exe");
        assert!(protocols_match(fish, Some(fish), Some(fish)));
        assert!(!protocols_match(fish, Some(fish), Some(blox)));
        assert!(!protocols_match(fish, Some(blox), Some(fish)));
        assert!(!protocols_match(fish, Some(fish), None));
    }
    #[test]
    fn no_provider_can_substitute_for_network_only() {
        for bootstrapper in [
            Bootstrapper::System,
            Bootstrapper::Official,
            Bootstrapper::Bloxstrap,
            Bootstrapper::Fishstrap,
            Bootstrapper::Custom,
        ] {
            let settings = ClientSettings {
                bootstrapper,
                mode: crate::client::ClientMode::NetworkOnly,
                ..Default::default()
            };
            assert!(bootstrapper.request(&settings).is_err());
        }
    }
    #[test]
    fn invalid_executables_are_rejected() {
        assert!(verify_executable(Path::new("relative.exe"), Bootstrapper::Custom).is_err());
        assert!(
            verify_executable(
                &std::env::temp_dir().join("cmd.exe"),
                Bootstrapper::Official
            )
            .is_err()
        );
        #[cfg(windows)]
        {
            let executable = std::env::current_exe().unwrap();
            let settings = ClientSettings {
                bootstrapper: Bootstrapper::Custom,
                executable: Some(executable.to_string_lossy().into_owned()),
                ..Default::default()
            };
            let request = Bootstrapper::Custom.request(&settings).unwrap();
            assert!(
                matches!(request, LaunchRequest::Executable(path) if path == std::fs::canonicalize(executable).unwrap())
            );
            assert!(matches!(
                Bootstrapper::System
                    .request(&ClientSettings::default())
                    .unwrap(),
                LaunchRequest::System
            ));
        }
    }
}
