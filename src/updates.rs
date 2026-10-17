//! Release discovery; installers are validated and applied by the desktop updater.
use serde::{Deserialize, Serialize};

pub const DEFAULT_REPOSITORY: &str = "OHmMieKunG/RoLauncher";

pub fn repository_or_default(repository: &str) -> &str {
    if repository.trim().is_empty() {
        DEFAULT_REPOSITORY
    } else {
        repository
    }
}

pub fn validate_repository(repository: &str) -> Result<(), String> {
    let parts = repository.split('/').collect::<Vec<_>>();
    if parts.len() != 2
        || parts.iter().any(|p| {
            p.is_empty()
                || p.len() > 100
                || matches!(*p, "." | "..")
                || !p
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
        })
    {
        return Err("Enter a public GitHub repository as owner/repository".into());
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Version {
    core: (u64, u64, u64),
    channel: u8,
    beta: u64,
}
fn version(value: &str) -> Result<Version, String> {
    let v = value.strip_prefix('v').unwrap_or(value);
    let (base, channel, beta) = if let Some((base, beta)) = v.split_once("-beta.") {
        if beta.is_empty() || beta.starts_with('0') || !beta.bytes().all(|c| c.is_ascii_digit()) {
            return Err("Invalid beta version".into());
        }
        (base, 1, beta.parse().map_err(|_| "Invalid beta version")?)
    } else if let Some((base, hash)) = v.split_once("-dev.") {
        if hash.len() != 12
            || !hash
                .bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        {
            return Err("Invalid development version".into());
        }
        (base, 0, 0)
    } else {
        (v, 2, 0)
    };
    let parts = base.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts.iter().any(|p| {
            p.is_empty()
                || p.len() > 1 && p.starts_with('0')
                || !p.bytes().all(|c| c.is_ascii_digit())
        })
    {
        return Err("Release tag must be MAJOR.MINOR.PATCH".into());
    }
    Ok(Version {
        core: (
            parts[0].parse().map_err(|_| "Invalid release version")?,
            parts[1].parse().map_err(|_| "Invalid release version")?,
            parts[2].parse().map_err(|_| "Invalid release version")?,
        ),
        channel,
        beta,
    })
}
fn newer_than_current(candidate: Version, current: &str) -> Result<bool, String> {
    Ok(candidate > version(current)?)
}
#[derive(Deserialize)]
pub struct RepositoryPatch {
    pub repository: String,
}
#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}
#[derive(Serialize)]
pub struct UpdateView {
    pub available: bool,
    pub version: String,
    pub release_url: String,
    pub download_url: String,
    pub checksums_url: String,
    pub installer_url: String,
    pub prerelease: bool,
}
fn release_version(value: &Release, include_beta: bool) -> Result<Version, String> {
    if value.draft || (value.prerelease && !include_beta) {
        return Err("This release is excluded from the selected update channel".into());
    }
    if value.prerelease {
        if let Some(legacy) = value.tag_name.strip_prefix("beta-v") {
            let mut parsed = version(legacy)?;
            if parsed.channel != 2 {
                return Err("Unsupported legacy beta tag".into());
            }
            parsed.channel = 1;
            Ok(parsed)
        } else {
            let parsed = version(&value.tag_name)?;
            if parsed.channel != 1 {
                return Err("Unsupported beta release tag".into());
            }
            Ok(parsed)
        }
    } else {
        let parsed = version(&value.tag_name)?;
        if parsed.channel != 2 {
            return Err("Stable release requires a numeric version".into());
        }
        Ok(parsed)
    }
}
fn parse_release(repo: &str, value: Release, include_beta: bool) -> Result<UpdateView, String> {
    validate_repository(repo)?;
    let number = release_version(&value, include_beta)?;
    let release_version = value
        .tag_name
        .strip_prefix("beta-v")
        .or_else(|| value.tag_name.strip_prefix('v'))
        .unwrap_or(&value.tag_name)
        .to_owned();
    let asset = |suffix: &str| -> Result<String, String> {
        let name = format!("rolauncher-v{release_version}-{suffix}");
        let expected = format!(
            "https://github.com/{repo}/releases/download/{}/{name}",
            value.tag_name
        );
        let entry = value
            .assets
            .iter()
            .find(|a| a.name == name && a.browser_download_url == expected)
            .ok_or("Release is missing a matching platform package or checksum file")?;
        Ok(entry.browser_download_url.clone())
    };
    let download_url = asset(package_suffix())?;
    let checksums_url = asset("SHA256SUMS.txt")?;
    let installer_url = asset("setup-x64.exe")?;
    Ok(UpdateView {
        available: newer_than_current(number, env!("CARGO_PKG_VERSION"))?,
        version: release_version,
        release_url: format!("https://github.com/{repo}/releases/tag/{}", value.tag_name),
        download_url,
        checksums_url,
        installer_url,
        prerelease: value.prerelease,
    })
}
fn package_suffix() -> &'static str {
    "windows-x64.zip"
}
fn select_release(repo: &str, releases: Vec<Release>) -> Result<UpdateView, String> {
    // Compare numeric versions rather than GitHub publication order. Prefer a stable
    // release when both channels publish the same product version.
    let release = releases
        .into_iter()
        .filter_map(|release| {
            release_version(&release, true)
                .ok()
                .map(|number| (number, !release.prerelease, release))
        })
        .max_by_key(|(number, stable, _)| (*number, *stable))
        .ok_or("No published release matches the selected update channel")?
        .2;
    parse_release(repo, release, true)
}
pub async fn check(repository: &str, include_beta: bool) -> Result<UpdateView, String> {
    let repository = repository_or_default(repository);
    validate_repository(repository)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(20))
        .user_agent(concat!("RoLauncher/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| "Unable to initialize update check")?;
    let endpoint = if include_beta {
        "releases?per_page=100"
    } else {
        "releases/latest"
    };
    let response = client
        .get(format!(
            "https://api.github.com/repos/{repository}/{endpoint}"
        ))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2026-03-10")
        .send()
        .await
        .map_err(|_| "Unable to reach GitHub for update information")?;
    if !response.status().is_success() {
        return Err(format!(
            "Update check failed (HTTP {}); check the repository or try later",
            response.status().as_u16()
        ));
    }
    // Stream with an explicit cap, including when Content-Length is absent.
    let mut response = response;
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Unable to read update information")?
    {
        if bytes.len() + chunk.len() > 1024 * 1024 {
            return Err("Update response is too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    if include_beta {
        select_release(
            repository,
            serde_json::from_slice(&bytes).map_err(|_| "Unsupported release response")?,
        )
    } else {
        parse_release(
            repository,
            serde_json::from_slice(&bytes).map_err(|_| "Unsupported release response")?,
            false,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(tag: &str, prerelease: bool) -> Release {
        let number = tag
            .strip_prefix("beta-v")
            .or_else(|| tag.strip_prefix('v'))
            .unwrap_or(tag);
        Release {
            tag_name: tag.into(),
            draft: false,
            prerelease,
            assets: [
                "windows-x64.zip",
                "linux-x64.tar.gz",
                "setup-x64.exe",
                "SHA256SUMS.txt",
            ]
            .into_iter()
            .map(|suffix| {
                let name = format!("rolauncher-v{number}-{suffix}");
                Asset {
                    browser_download_url: format!(
                        "https://github.com/owner/repo/releases/download/{tag}/{name}"
                    ),
                    name,
                }
            })
            .collect(),
        }
    }
    #[test]
    fn beta_updates_require_opt_in_and_exact_beta_asset_links() {
        assert!(parse_release("owner/repo", fixture("beta-v1.10.0", true), false).is_err());
        let beta = parse_release("owner/repo", fixture("beta-v1.10.0", true), true).unwrap();
        assert!(beta.prerelease && beta.available);
        assert_eq!(beta.version, "1.10.0");
        assert!(beta.checksums_url.contains("/beta-v1.10.0/"));
        assert!(parse_release("owner/repo", fixture("beta-v1.10.0", false), true).is_err());
        assert!(parse_release("owner/repo", fixture("v1.10.0", true), true).is_err());
        let mut wrong = fixture("beta-v1.10.0", true);
        wrong.assets[0].browser_download_url = wrong.assets[0]
            .browser_download_url
            .replace("/beta-v", "/v");
        wrong.assets[1].browser_download_url = wrong.assets[1]
            .browser_download_url
            .replace("/beta-v", "/v");
        assert!(parse_release("owner/repo", wrong, true).is_err());
    }
    #[test]
    fn beta_channel_selects_numeric_newest_and_prefers_stable_for_ties() {
        let mut draft = fixture("beta-v99.0.0", true);
        draft.draft = true;
        let newest = select_release(
            "owner/repo",
            vec![
                fixture("v1.9.9", false),
                fixture("beta-v1.10.0", true),
                draft,
            ],
        )
        .unwrap();
        assert_eq!(newest.version, "1.10.0");
        assert!(newest.prerelease);
        let stable = select_release(
            "owner/repo",
            vec![fixture("v1.10.0", false), fixture("beta-v1.10.0", true)],
        )
        .unwrap();
        assert!(!stable.prerelease);
        // A broken newest release must fail instead of offering an older download.
        let mut missing = fixture("beta-v1.11.0", true);
        missing.assets.clear();
        assert!(select_release("owner/repo", vec![fixture("v1.10.0", false), missing]).is_err());
        assert!(select_release("owner/repo", vec![]).is_err());
    }
    #[test]
    fn repository_validation_cannot_escape_the_github_endpoint() {
        for value in [
            "https://evil.example",
            "owner/../repo",
            "owner/..",
            "/repo",
            "owner/repo?x=1",
            "owner/repo#x",
        ] {
            assert!(validate_repository(value).is_err());
        }
        assert!(validate_repository("owner/rolauncher").is_ok());
    }
    #[test]
    fn updates_compare_numeric_versions_and_require_matching_package_links() {
        let make = || {
            Release { tag_name: "v1.10.0".into(), draft: false, prerelease: false, assets: ["windows-x64.zip", "linux-x64.tar.gz", "setup-x64.exe", "SHA256SUMS.txt"].iter().map(|suffix| Asset { name: format!("rolauncher-v1.10.0-{suffix}"), browser_download_url: format!("https://github.com/owner/repo/releases/download/v1.10.0/rolauncher-v1.10.0-{suffix}") }).collect() }
        };
        let view = parse_release("owner/repo", make(), false).unwrap();
        assert!(view.available);
        assert_eq!(view.version, "1.10.0");
        assert!(
            view.installer_url
                .ends_with("rolauncher-v1.10.0-setup-x64.exe")
        );
        assert!(view.download_url.ends_with(package_suffix()));
        assert_eq!(repository_or_default(""), DEFAULT_REPOSITORY);
        assert!(version("1.10.0").unwrap() > version("1.9.9").unwrap());
        assert!(version("1.1.0-beta").is_err());
        assert!(newer_than_current(version("1.4.0").unwrap(), "1.4.0-dev.123456abcdef").unwrap());
        assert!(!newer_than_current(version("1.3.0").unwrap(), "1.4.0-dev.123456abcdef").unwrap());
        assert!(!newer_than_current(version("1.4.0").unwrap(), "1.4.0").unwrap());
        assert!(newer_than_current(version("1.5.0").unwrap(), "1.4.0-dev.123456abcdef").unwrap());
        assert!(newer_than_current(version("1.4.0").unwrap(), "1.4.0-dev.invalid").is_err());
        let mut bad = make();
        bad.assets
            .iter_mut()
            .find(|a| a.name.ends_with(package_suffix()))
            .unwrap()
            .browser_download_url = "https://evil.example/malware.zip".into();
        assert!(parse_release("owner/repo", bad, false).is_err());
        let mut draft = make();
        draft.draft = true;
        assert!(parse_release("owner/repo", draft, true).is_err());
        let mut missing = make();
        missing.assets.pop();
        assert!(parse_release("owner/repo", missing, false).is_err());
    }
    #[test]
    fn numbered_betas_sort_numerically_and_stable_replaces_matching_beta_and_dev() {
        let beta = version("1.8.4-beta.10").unwrap();
        assert!(newer_than_current(beta, "1.8.4-beta.2").unwrap());
        assert!(newer_than_current(beta, "1.8.4-dev.123456abcdef").unwrap());
        assert!(!newer_than_current(beta, "1.8.4").unwrap());
        assert!(newer_than_current(version("1.8.4").unwrap(), "1.8.4-beta.10").unwrap());
        for bad in [
            "1.8.4-beta.0",
            "1.8.4-beta.01",
            "1.8.4-beta.-1",
            "1.8.4-beta.1+extra",
        ] {
            assert!(version(bad).is_err());
        }
        let selected = select_release(
            "owner/repo",
            vec![
                fixture("v1.8.4-beta.2", true),
                fixture("v1.8.4-beta.10", true),
            ],
        )
        .unwrap();
        assert_eq!(selected.version, "1.8.4-beta.10");
        assert!(
            selected
                .installer_url
                .contains("v1.8.4-beta.10/rolauncher-v1.8.4-beta.10-")
        );
        assert!(parse_release("owner/repo", fixture("v1.8.4-beta.1", true), false).is_err());
        assert!(parse_release("owner/repo", fixture("v1.8.4-beta.1", false), true).is_err());
    }
}
