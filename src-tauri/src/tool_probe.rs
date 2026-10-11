use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use crate::process_utils::hidden_command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileIdentity {
    path: PathBuf,
    length: u64,
    modified: SystemTime,
}

impl FileIdentity {
    pub(crate) fn read(path: &Path) -> Option<Self> {
        let path = path.canonicalize().ok()?;
        let metadata = path.metadata().ok()?;
        metadata.is_file().then_some(Self {
            path,
            length: metadata.len(),
            modified: metadata.modified().ok()?,
        })
    }

    pub(crate) fn read_executable(program: &Path) -> Option<Self> {
        Self::read(&crate::tool_paths::resolve_executable(program)?)
    }
}

struct CachedVersion {
    identity: FileIdentity,
    checked_at: Instant,
    version: String,
}

#[derive(Default)]
struct VersionCache {
    // Serialize checks for the same tool, while different tools run concurrently.
    entries: Mutex<HashMap<(PathBuf, String), Arc<tokio::sync::Mutex<Option<CachedVersion>>>>>,
}

impl VersionCache {
    async fn check(&self, program: PathBuf, arg: &str) -> Option<String> {
        let Some(identity) = FileIdentity::read_executable(&program) else {
            // Missing/unresolvable executables cannot have a cached identity.
            return probe(program, arg).await;
        };
        let entry = {
            let mut entries = self.entries.lock().ok()?;
            if entries.len() >= 16 && !entries.contains_key(&(identity.path.clone(), arg.into())) {
                entries.clear();
            }
            entries
                .entry((identity.path.clone(), arg.into()))
                .or_default()
                .clone()
        };
        let mut cached = entry.lock().await;
        if let Some(value) = cached.as_ref() {
            if value.identity == identity && value.checked_at.elapsed() < Duration::from_secs(60) {
                return Some(value.version.clone());
            }
        }
        let version = probe(identity.path.clone(), arg).await?;
        *cached = Some(CachedVersion {
            identity,
            checked_at: Instant::now(),
            version: version.clone(),
        });
        Some(version)
    }
}

pub(crate) async fn command_version(program: PathBuf, arg: &str) -> Option<String> {
    static CACHE: OnceLock<VersionCache> = OnceLock::new();
    CACHE
        .get_or_init(VersionCache::default)
        .check(program, arg)
        .await
}

async fn probe(program: PathBuf, arg: &str) -> Option<String> {
    let mut command = hidden_command(program);
    command.kill_on_drop(true).arg(arg).stdin(Stdio::null());
    let output = tokio::time::timeout(Duration::from_secs(5), command.output())
        .await
        .ok()?
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let bytes = if output.stdout.is_empty() {
        &output.stderr
    } else {
        &output.stdout
    };
    String::from_utf8_lossy(bytes)
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn repeated_checks_reuse_the_process_result_and_expire() {
        let cache = VersionCache::default();
        // The Rust test runner is a real executable with a successful --list command.
        let executable = std::env::current_exe().unwrap();
        let first = cache.check(executable.clone(), "--list").await.unwrap();
        let entry = cache
            .entries
            .lock()
            .unwrap()
            .values()
            .next()
            .unwrap()
            .clone();
        entry.lock().await.as_mut().unwrap().version = "cached result".into();
        assert_eq!(
            cache.check(executable.clone(), "--list").await.as_deref(),
            Some("cached result")
        );
        entry.lock().await.as_mut().unwrap().checked_at = Instant::now() - Duration::from_secs(61);
        assert_eq!(
            cache.check(executable, "--list").await.as_deref(),
            Some(first.as_str())
        );
    }

    #[test]
    fn replacement_and_removal_invalidate_file_identity() {
        let path = std::env::temp_dir().join(format!("ydlite-probe-{}", uuid::Uuid::new_v4()));
        std::fs::write(&path, b"first").unwrap();
        let first = FileIdentity::read(&path).unwrap();
        std::fs::write(&path, b"replacement").unwrap();
        assert_ne!(FileIdentity::read(&path).unwrap(), first);
        std::fs::remove_file(&path).unwrap();
        assert!(FileIdentity::read(&path).is_none());
    }

    #[tokio::test]
    #[ignore = "requires the installed yt-dlp executable"]
    async fn benchmark_real_tool_version_cache() {
        let program =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/tools/yt-dlp.exe");
        let cache = VersionCache::default();
        let start = Instant::now();
        let version = cache.check(program.clone(), "--version").await.unwrap();
        let cold = start.elapsed();
        let start = Instant::now();
        assert_eq!(
            cache.check(program, "--version").await.as_ref(),
            Some(&version)
        );
        println!(
            "Real tool version: first {:.3}s, cached {:.6}s",
            cold.as_secs_f64(),
            start.elapsed().as_secs_f64()
        );
    }
}
