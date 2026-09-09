//! Explicit, standalone maintenance updates.
//!
//! This module is intentionally outside the mutation pipeline. Mutation and MCP operations do
//! not call it and do not gain network capability as a result of its existence.

use self_update::{backends::github, Release, ReleaseAsset};
use semver::Version;
use serde::Serialize;
use std::{path::Path, process::Command};

const OWNER: &str = "matthewjameswatkins1978-cyber";
const REPOSITORY: &str = "Threadmoth";
const RELEASE_MANIFEST_ASSET: &str = "release-manifest.json";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Platform {
    pub name: &'static str,
    pub archive: &'static str,
    pub executable: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallationKind {
    Standalone,
    CargoManaged,
    HomebrewManaged,
    WingetManaged,
    Ambiguous,
}

impl InstallationKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Standalone => "standalone",
            Self::CargoManaged => "cargo-managed",
            Self::HomebrewManaged => "homebrew-managed",
            Self::WingetManaged => "winget-managed",
            Self::Ambiguous => "ambiguous",
        }
    }

    pub fn is_standalone(self) -> bool {
        matches!(self, Self::Standalone)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct UpdateReport {
    pub status: String,
    pub current_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_version: Option<String>,
    pub platform: String,
    pub installation: String,
    pub verification: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl UpdateReport {
    pub fn refused(
        installation: InstallationKind,
        _kind: UpdateErrorKind,
        message: impl Into<String>,
    ) -> Self {
        let platform =
            platform().map_or_else(|_| "unsupported".to_string(), |p| p.name.to_string());
        Self {
            status: "refused".to_string(),
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            available_version: None,
            platform,
            installation: installation.label().to_string(),
            verification: "not started".to_string(),
            error: Some(message.into()),
        }
    }

    pub fn from_error(error: UpdateError) -> Self {
        Self {
            status: error.kind.status().to_string(),
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            available_version: error.available_version,
            platform: error.platform.unwrap_or_else(|| "unknown".to_string()),
            installation: installation_kind().label().to_string(),
            verification: error.verification,
            error: Some(error.message),
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self.status.as_str() {
            "refused" => 2,
            "failed" => 3,
            _ => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateErrorKind {
    UnsupportedInstall,
    PlatformUnsupported,
    AssetNotFound,
    VersionInvalid,
    IntegrityFailed,
    Network,
    Failed,
}

impl UpdateErrorKind {
    fn status(self) -> &'static str {
        match self {
            Self::UnsupportedInstall
            | Self::PlatformUnsupported
            | Self::AssetNotFound
            | Self::VersionInvalid => "refused",
            Self::IntegrityFailed | Self::Network | Self::Failed => "failed",
        }
    }
}

#[derive(Debug)]
pub struct UpdateError {
    pub kind: UpdateErrorKind,
    pub message: String,
    pub available_version: Option<String>,
    pub platform: Option<String>,
    pub verification: String,
}

impl UpdateError {
    fn new(kind: UpdateErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            available_version: None,
            platform: None,
            verification: "not started".to_string(),
        }
    }

    fn with_platform(mut self, platform: &Platform) -> Self {
        self.platform = Some(platform.name.to_string());
        self
    }
}

#[derive(Clone, Debug)]
pub struct UpdateInfo {
    pub current_version: String,
    pub available_version: Option<String>,
    pub platform: Platform,
    pub installation: InstallationKind,
    pub artifact: String,
    pub github_digest_available: bool,
}

impl UpdateInfo {
    pub fn is_current(&self) -> bool {
        self.available_version.is_none()
    }

    pub fn into_report(self, status: &str) -> UpdateReport {
        UpdateReport {
            status: status.to_string(),
            current_version: self.current_version,
            available_version: self.available_version,
            platform: self.platform.name.to_string(),
            installation: self.installation.label().to_string(),
            verification: verification_label(self.github_digest_available),
            error: None,
        }
    }
}

#[allow(unreachable_code)]
pub fn platform() -> Result<Platform, UpdateError> {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return Ok(Platform {
            name: "windows-x86_64",
            archive: "threadmoth-windows-x86_64.zip",
            executable: "threadmoth.exe",
        });
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return Ok(Platform {
            name: "linux-x86_64",
            archive: "threadmoth-linux-x86_64.tar.gz",
            executable: "threadmoth",
        });
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return Ok(Platform {
            name: "macos-aarch64",
            archive: "threadmoth-macos-aarch64.tar.gz",
            executable: "threadmoth",
        });
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return Ok(Platform {
            name: "macos-x86_64",
            archive: "threadmoth-macos-x86_64.tar.gz",
            executable: "threadmoth",
        });
    }
    Err(UpdateError::new(
        UpdateErrorKind::PlatformUnsupported,
        "no official Threadmoth release artifact exists for this platform",
    ))
}

pub fn installation_kind() -> InstallationKind {
    let path = std::env::current_exe().unwrap_or_default();
    classify_installation(&path)
}

fn classify_installation(path: &Path) -> InstallationKind {
    let lower = path
        .to_string_lossy()
        .to_ascii_lowercase()
        .replace('/', "\\");
    if lower.contains("\\.cargo\\bin\\") {
        InstallationKind::CargoManaged
    } else if lower.contains("\\cellar\\")
        || lower.contains("\\homebrew\\")
        || lower.contains("/opt/homebrew/")
    {
        InstallationKind::HomebrewManaged
    } else if lower.contains("\\winget\\") || lower.contains("\\windowsapps\\") {
        InstallationKind::WingetManaged
    } else {
        InstallationKind::Standalone
    }
}

pub fn discover(requested: Option<&str>) -> Result<UpdateInfo, UpdateError> {
    let platform = platform()?;
    let installation = installation_kind();
    if !installation.is_standalone() {
        return Err(UpdateError::new(
            UpdateErrorKind::UnsupportedInstall,
            format!(
                "installation appears {}; update it with its package manager",
                installation.label()
            ),
        )
        .with_platform(&platform));
    }

    let current = Version::parse(env!("CARGO_PKG_VERSION")).map_err(|error| {
        UpdateError::new(UpdateErrorKind::VersionInvalid, error.to_string())
            .with_platform(&platform)
    })?;
    let requested_version = requested.map(parse_stable_version).transpose()?;
    if let Some(version) = &requested_version {
        if version < &current {
            return Err(UpdateError::new(
                UpdateErrorKind::VersionInvalid,
                format!("refusing downgrade or reinstall from {current} to {version}"),
            )
            .with_platform(&platform));
        }
        if version == &current {
            return Ok(UpdateInfo {
                current_version: current.to_string(),
                available_version: None,
                platform,
                installation,
                artifact: String::new(),
                github_digest_available: false,
            });
        }
    }

    let release = fetch_release(requested_version.as_ref(), &platform)?;
    let release_version = parse_stable_version(release.version()).map_err(|mut error| {
        error.platform = Some(platform.name.to_string());
        error
    })?;
    if release_version <= current {
        return Ok(UpdateInfo {
            current_version: current.to_string(),
            available_version: None,
            platform,
            installation,
            artifact: String::new(),
            github_digest_available: false,
        });
    }
    let artifact = exact_asset(&release, platform.archive).ok_or_else(|| {
        UpdateError::new(
            UpdateErrorKind::AssetNotFound,
            format!(
                "release {} does not contain exactly one {} asset",
                release.version(),
                platform.archive
            ),
        )
        .with_platform(&platform)
    })?;
    let checksum_asset = checksum_asset_name(&platform);
    if exact_asset(&release, RELEASE_MANIFEST_ASSET).is_none()
        || exact_asset(&release, &checksum_asset).is_none()
    {
        return Err(UpdateError::new(
            UpdateErrorKind::IntegrityFailed,
            format!(
                "release {} does not contain exactly one release manifest and {} checksum asset",
                release.version(),
                checksum_asset
            ),
        )
        .with_platform(&platform));
    }

    Ok(UpdateInfo {
        current_version: current.to_string(),
        available_version: Some(release_version.to_string()),
        platform,
        installation,
        artifact: artifact.name().to_string(),
        github_digest_available: artifact.digest().is_some(),
    })
}

pub fn install(info: &UpdateInfo) -> Result<UpdateReport, UpdateError> {
    let version = info
        .available_version
        .as_deref()
        .ok_or_else(|| UpdateError::new(UpdateErrorKind::Failed, "no update is available"))?;
    let expected_artifact = info.artifact.clone();
    let expected_version = version.to_string();
    let executable = info.platform.executable.to_string();
    let mut builder = github::Update::configure();
    builder
        .repo_owner(OWNER)
        .repo_name(REPOSITORY)
        .target(info.platform.name)
        .bin_name("threadmoth")
        .bin_path_in_archive(&executable)
        .current_version(&info.current_version)
        .release_tag(format!("v{expected_version}"))
        .asset_matcher(move |assets| {
            let mut matches = assets
                .iter()
                .filter(|asset| asset.name() == expected_artifact);
            let selected = matches.next().cloned();
            if matches.next().is_some() {
                None
            } else {
                selected
            }
        })
        .checksum_from_asset(checksum_asset_name(&info.platform))
        .verify_release_digest(true)
        .check_install_path_writable(true)
        .unattended();
    let expected_for_binary = expected_version.clone();
    builder.verify_binary(move |path| verify_binary(path, &expected_for_binary));
    let updater = builder.build().map_err(|error| {
        UpdateError::new(UpdateErrorKind::Failed, error.to_string()).with_platform(&info.platform)
    })?;
    updater.update_extended().map_err(|error| {
        let kind = if error.to_string().to_ascii_lowercase().contains("checksum")
            || error.to_string().to_ascii_lowercase().contains("digest")
        {
            UpdateErrorKind::IntegrityFailed
        } else {
            UpdateErrorKind::Failed
        };
        UpdateError {
            kind,
            message: error.to_string(),
            available_version: Some(expected_version.clone()),
            platform: Some(info.platform.name.to_string()),
            verification: verification_label(info.github_digest_available),
        }
    })?;
    Ok(UpdateReport {
        status: "updated".to_string(),
        current_version: info.current_version.clone(),
        available_version: Some(expected_version),
        platform: info.platform.name.to_string(),
        installation: info.installation.label().to_string(),
        verification: verification_label(info.github_digest_available),
        error: None,
    })
}

fn fetch_release(requested: Option<&Version>, platform: &Platform) -> Result<Release, UpdateError> {
    let mut builder = github::Update::configure();
    builder
        .repo_owner(OWNER)
        .repo_name(REPOSITORY)
        .target(platform.name)
        .bin_name("threadmoth")
        .current_version(env!("CARGO_PKG_VERSION"))
        .unattended();
    if let Some(version) = requested {
        builder.release_tag(format!("v{version}"));
    }
    let updater = builder.build().map_err(|error| {
        UpdateError::new(UpdateErrorKind::Network, error.to_string()).with_platform(platform)
    })?;
    if let Some(version) = requested {
        updater
            .get_release_version(&format!("v{version}"))
            .map_err(|error| {
                UpdateError::new(UpdateErrorKind::Network, error.to_string())
                    .with_platform(platform)
            })
    } else {
        let releases = updater.get_latest_release().map_err(|error| {
            UpdateError::new(UpdateErrorKind::Network, error.to_string()).with_platform(platform)
        })?;
        releases.latest().cloned().ok_or_else(|| {
            UpdateError::new(UpdateErrorKind::AssetNotFound, "no stable release found")
                .with_platform(platform)
        })
    }
}

fn parse_stable_version(value: &str) -> Result<Version, UpdateError> {
    let normalized = value.strip_prefix('v').unwrap_or(value);
    let version = Version::parse(normalized).map_err(|error| {
        UpdateError::new(
            UpdateErrorKind::VersionInvalid,
            format!("invalid release version {value}: {error}"),
        )
    })?;
    if !version.pre.is_empty() {
        return Err(UpdateError::new(
            UpdateErrorKind::VersionInvalid,
            format!("prerelease versions are not supported: {value}"),
        ));
    }
    Ok(version)
}

fn exact_asset<'a>(release: &'a Release, name: &str) -> Option<&'a ReleaseAsset> {
    let mut matches = release.assets().iter().filter(|asset| asset.name() == name);
    let selected = matches.next();
    if matches.next().is_some() {
        None
    } else {
        selected
    }
}

fn verify_binary(path: &Path, expected_version: &str) -> self_update::Result<()> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|error| self_update::Error::verification_rejected(error.to_string()))?;
    if !output.status.success() {
        return Err(self_update::Error::verification_rejected(
            "extracted Threadmoth executable did not run successfully",
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.contains(expected_version) {
        return Err(self_update::Error::verification_rejected(format!(
            "extracted executable reported {:?}, expected {}",
            stdout.trim(),
            expected_version
        )));
    }
    Ok(())
}

fn verification_label(github_digest: bool) -> String {
    if github_digest {
        "SHA-256 checksum + GitHub digest (release manifest present)".to_string()
    } else {
        "SHA-256 checksum (GitHub digest unavailable; release manifest present)".to_string()
    }
}

fn checksum_asset_name(platform: &Platform) -> String {
    let archive_stem = platform
        .archive
        .strip_suffix(".tar.gz")
        .or_else(|| platform.archive.strip_suffix(".zip"))
        .unwrap_or(platform.archive);
    format!("{archive_stem}.sha256")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_version_rejects_prerelease_and_invalid_values() {
        assert!(parse_stable_version("1.8.0").is_ok());
        assert!(parse_stable_version("v1.8.0").is_ok());
        assert_eq!(
            parse_stable_version("1.8.0-rc1").unwrap_err().kind,
            UpdateErrorKind::VersionInvalid
        );
        assert_eq!(
            parse_stable_version("latest").unwrap_err().kind,
            UpdateErrorKind::VersionInvalid
        );
    }

    #[test]
    fn package_paths_are_not_self_updated() {
        assert_eq!(
            classify_installation(Path::new(r"C:\Users\m\.cargo\bin\threadmoth.exe")),
            InstallationKind::CargoManaged
        );
        assert_eq!(
            classify_installation(Path::new(r"C:\Program Files\WindowsApps\threadmoth.exe")),
            InstallationKind::WingetManaged
        );
        assert_eq!(
            classify_installation(Path::new(r"C:\Tools\threadmoth.exe")),
            InstallationKind::Standalone
        );
        assert_eq!(
            classify_installation(Path::new(r"C:\Program Files\Threadmoth\threadmoth.exe")),
            InstallationKind::Standalone
        );
    }

    #[test]
    fn exact_asset_selection_is_not_ambiguous() {
        let release = Release::builder()
            .version("1.8.0")
            .assets(vec![
                ReleaseAsset::new("threadmoth-windows-x86_64.zip", "https://example/a"),
                ReleaseAsset::new("release-manifest.json", "https://example/m"),
            ])
            .build()
            .unwrap();
        assert_eq!(
            exact_asset(&release, "threadmoth-windows-x86_64.zip")
                .unwrap()
                .name(),
            "threadmoth-windows-x86_64.zip"
        );
        assert!(exact_asset(&release, "missing.zip").is_none());
    }

    #[test]
    fn checksum_asset_matches_the_published_archive_name() {
        let windows = Platform {
            name: "windows-x86_64",
            archive: "threadmoth-windows-x86_64.zip",
            executable: "threadmoth.exe",
        };
        let unix = Platform {
            name: "linux-x86_64",
            archive: "threadmoth-linux-x86_64.tar.gz",
            executable: "threadmoth",
        };
        assert_eq!(
            checksum_asset_name(&windows),
            "threadmoth-windows-x86_64.sha256"
        );
        assert_eq!(checksum_asset_name(&unix), "threadmoth-linux-x86_64.sha256");
    }
}
