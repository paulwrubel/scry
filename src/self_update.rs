//! Replace the installed binaries from a GitHub release.

use std::{
    fs,
    io::{Read, Write},
    path::Path,
    time::Duration,
};

use flate2::read::GzDecoder;
use reqwest::Client;
use sha2::{Digest, Sha256};
use tar::Archive;

use crate::error::AppError;

const REPO: &str = "paulwrubel/scry";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Check for or install the latest release over the running binaries.
pub async fn run(check: bool) -> Result<(), AppError> {
    let target = release_target().ok_or_else(|| {
        AppError::Update(format!(
            "self-update does not support {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
    })?;
    // GitHub rejects requests without a User-Agent and reqwest sends none by default.
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent(format!("scry/{CURRENT_VERSION}"))
        .build()
        .map_err(|error| AppError::Update(format!("failed to build HTTP client: {error}")))?;
    let tag = latest_tag(&client).await?;
    let latest = parse_version(&tag)
        .ok_or_else(|| AppError::Update(format!("unrecognized release tag '{tag}'")))?;
    if parse_version(CURRENT_VERSION).is_some_and(|current| current >= latest) {
        println!("scry {CURRENT_VERSION} is up to date (latest release is {tag})");
        return Ok(());
    }
    if check {
        println!("scry {CURRENT_VERSION} can be updated to {tag}");
        return Ok(());
    }
    println!("Downloading scry {tag} for {target}...");
    let base = format!("https://github.com/{REPO}/releases/download/{tag}");
    let checksums = download(&client, &format!("{base}/checksums.txt")).await?;
    let archive = download(&client, &format!("{base}/scry-{target}.tar.gz")).await?;
    let asset = format!("scry-{target}.tar.gz");
    let expected = checksum_for(std::str::from_utf8(&checksums).unwrap_or_default(), &asset)
        .ok_or_else(|| AppError::Update(format!("release {tag} has no checksum for {asset}")))?;
    let actual = sha256_hex(&archive);
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(AppError::Update(format!(
            "checksum mismatch for {asset}: expected {expected}, got {actual}"
        )));
    }
    let (scry, api) = extract_binaries(&archive)?;
    let exe = std::env::current_exe()
        .map_err(|error| AppError::Update(format!("failed to locate scry: {error}")))?;
    let exe = fs::canonicalize(exe)
        .map_err(|error| AppError::Update(format!("failed to resolve scry: {error}")))?;
    let dir = exe.parent().ok_or_else(|| {
        AppError::Update(format!(
            "cannot find the install directory for {}",
            exe.display()
        ))
    })?;
    let api_path = dir.join("scry-api");
    let update_api = api_path.exists();
    let api = if update_api {
        Some(api.ok_or_else(|| {
            AppError::Update("release archive does not contain scry-api".to_string())
        })?)
    } else {
        None
    };
    install(&exe, &scry)?;
    if let Some(api) = api {
        install(&api_path, &api)?;
    }
    let names = if update_api {
        "scry and scry-api"
    } else {
        "scry"
    };
    println!("Updated {names} to {tag} in {}", dir.display());
    if update_api {
        println!("Restart scry-api for the new server build to take effect.");
    }
    Ok(())
}

/// The release triple for this platform, or `None` when none is published.
fn release_target() -> Option<&'static str> {
    match (
        std::env::consts::OS,
        std::env::consts::ARCH,
        cfg!(target_env = "musl"),
    ) {
        ("linux", "x86_64", false) => Some("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64", false) => Some("aarch64-unknown-linux-gnu"),
        ("macos", "x86_64", _) => Some("x86_64-apple-darwin"),
        ("macos", "aarch64", _) => Some("aarch64-apple-darwin"),
        _ => None,
    }
}

/// Parse a `v`-prefixed `major.minor.patch` tag with no extra components.
fn parse_version(value: &str) -> Option<(u64, u64, u64)> {
    let mut parts = value.strip_prefix('v').unwrap_or(value).split('.');
    let parse = |part: &str| part.parse::<u64>().ok();
    let version = (
        parse(parts.next()?)?,
        parse(parts.next()?)?,
        parse(parts.next()?)?,
    );
    parts.next().is_none().then_some(version)
}

/// Find the hash for `asset` in a sha256sum-style checksums file.
fn checksum_for<'a>(checksums: &'a str, asset: &str) -> Option<&'a str> {
    checksums.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        let hash = fields.next()?;
        let name = Path::new(fields.last()?).file_name()?.to_str()?;
        (name == asset).then_some(hash)
    })
}

/// Read the wanted binaries out of a gzipped tarball without touching the disk.
fn extract_binaries(archive: &[u8]) -> Result<(Vec<u8>, Option<Vec<u8>>), AppError> {
    let mut archive = Archive::new(GzDecoder::new(archive));
    let (mut scry, mut api): (Option<Vec<u8>>, Option<Vec<u8>>) = (None, None);
    let entries = archive
        .entries()
        .map_err(|error| AppError::Update(format!("failed to read the archive: {error}")))?;
    for entry in entries {
        let mut entry = entry.map_err(|error| {
            AppError::Update(format!("failed to read an archive entry: {error}"))
        })?;
        let name = entry.path().ok().and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        });
        let slot = match name.as_deref() {
            Some("scry") => &mut scry,
            Some("scry-api") => &mut api,
            _ => continue,
        };
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(|error| {
            AppError::Update(format!("failed to read a binary from the archive: {error}"))
        })?;
        *slot = Some(bytes);
    }
    scry.map(|scry| (scry, api)).ok_or_else(|| {
        AppError::Update("release archive does not contain the scry binary".to_string())
    })
}

/// Write to a sibling temp file, then rename over `dest` so a running binary survives.
fn install(dest: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let name = dest
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    // The temp must share dest's directory so the rename stays atomic.
    let temp = dest.with_file_name(format!(".{name}.new.{}", std::process::id()));
    let result = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .and_then(|mut file| {
            file.write_all(bytes)?;
            file.sync_all()
        })
        .and_then(|()| {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&temp, fs::Permissions::from_mode(0o755))?;
            }
            Ok(())
        })
        .and_then(|()| fs::rename(&temp, dest));
    if let Err(error) = result {
        let _ = fs::remove_file(&temp);
        return Err(AppError::Update(format!(
            "failed to install to {}: {error}; the directory may not be writable",
            dest.display()
        )));
    }
    Ok(())
}

/// Fetch the tag of the latest GitHub release.
async fn latest_tag(client: &Client) -> Result<String, AppError> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let response = client.get(&url).send().await.map_err(|error| {
        AppError::Update(format!("failed to fetch the latest release: {error}"))
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(AppError::Update(format!(
            "GitHub returned {status} for {url}"
        )));
    }
    let body = response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| AppError::Update(format!("failed to decode the release: {error}")))?;
    body.get("tag_name")
        .and_then(|tag| tag.as_str())
        .map(str::to_string)
        .ok_or_else(|| AppError::Update("latest release is missing a tag_name".to_string()))
}

/// Fetch a URL, rejecting non-2xx responses.
async fn download(client: &Client, url: &str) -> Result<Vec<u8>, AppError> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| AppError::Update(format!("failed to download {url}: {error}")))?;
    let status = response.status();
    if !status.is_success() {
        return Err(AppError::Update(format!(
            "failed to download {url}: server returned {status}"
        )));
    }
    response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|error| AppError::Update(format!("failed to read {url}: {error}")))
}

/// Lowercase hex SHA-256, avoiding a hex dependency.
fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
