//! The log files as Settings → Diagnostics shows them: their size, their last lines, one file of
//! all of them to send, and clearing them — what iOS does in `LogCollector`. The files are the
//! daily ones `init_logging` writes (`konstruct.<date>.log`); message text is never in them.

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// How much of the newest file is read for its last lines.
const TAIL_BYTES: u64 = 64 * 1024;

/// The log files in `dir`, oldest first: their names carry the date.
fn files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("konstruct") && n.ends_with(".log"))
        })
        .collect();
    files.sort();
    files
}

/// Bytes the logs take.
pub fn size(dir: &Path) -> u64 {
    files(dir)
        .iter()
        .filter_map(|p| p.metadata().ok())
        .map(|m| m.len())
        .sum()
}

/// The last `lines` lines written, oldest first.
pub fn recent(dir: &Path, lines: usize) -> String {
    let Some(newest) = files(dir).pop() else {
        return String::new();
    };
    let Ok(mut file) = fs::File::open(newest) else {
        return String::new();
    };
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    let from = len.saturating_sub(TAIL_BYTES);
    let mut bytes = Vec::new();
    if file.seek(SeekFrom::Start(from)).is_err() || file.read_to_end(&mut bytes).is_err() {
        return String::new();
    }
    let text = String::from_utf8_lossy(&bytes);
    // A read from the middle starts inside a line.
    let text = if from > 0 {
        text.split_once('\n').map_or("", |(_, rest)| rest)
    } else {
        &text
    };
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

/// Every log file, oldest first, in one file in `out` under a header saying what wrote them, as
/// iOS names its own (`construct-logs-<unix seconds>.txt`). Returns its path.
pub fn bundle(dir: &Path, out: &Path, now_secs: u64) -> Result<PathBuf, String> {
    let mut text = format!(
        "========================================\n\
         Konstruct Desktop logs\n\
         ========================================\n\
         Exported (unix seconds): {now_secs}\n\
         App version: {}\n\
         System: {}\n\
         ========================================\n\n",
        env!("CARGO_PKG_VERSION"),
        system(),
    );
    let files = files(dir);
    for (i, file) in files.iter().enumerate() {
        let name = file.file_name().unwrap_or_default().to_string_lossy();
        let content = fs::read(file).map_err(|e| format!("{name}: {e}"))?;
        text.push_str(&format!(
            "=== Log file {}/{}: {name} ===\n",
            i + 1,
            files.len()
        ));
        text.push_str(&String::from_utf8_lossy(&content));
        text.push_str("\n\n");
    }
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let path = out.join(format!("construct-logs-{now_secs}.txt"));
    fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Remove the logs. The newest file is the one being written to, opened for appending, so it is
/// emptied rather than removed: removed, it would take the next lines with it (and on Windows an
/// open file cannot be removed at all).
pub fn clear(dir: &Path) -> Result<(), String> {
    let mut files = files(dir);
    let Some(current) = files.pop() else {
        return Ok(());
    };
    for old in files {
        fs::remove_file(&old).map_err(|e| e.to_string())?;
    }
    fs::OpenOptions::new()
        .write(true)
        .open(&current)
        .and_then(|f| f.set_len(0))
        .map_err(|e| e.to_string())
}

/// The system, as a person reporting a problem would name it.
fn system() -> String {
    let base = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);
    // "Linux Mint 22", where the distribution says so.
    let pretty = fs::read_to_string("/etc/os-release").ok().and_then(|s| {
        s.lines()
            .find_map(|l| l.strip_prefix("PRETTY_NAME="))
            .map(|v| v.trim_matches('"').to_owned())
    });
    match pretty {
        Some(name) => format!("{name} ({base})"),
        None => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("konstruct-logs-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn day(dir: &Path, date: &str, text: &str) -> PathBuf {
        let p = dir.join(format!("konstruct.{date}.log"));
        fs::write(&p, text).unwrap();
        p
    }

    /// The file to send holds every day, oldest first, and nothing else from the folder.
    #[test]
    fn the_file_to_send_holds_every_day_in_order() {
        let d = dir("bundle");
        day(&d, "2026-10-25", "today\n");
        day(&d, "2026-10-24", "yesterday\n");
        fs::write(d.join("notes.txt"), "not a log").unwrap();
        let out = d.join("out");
        let path = bundle(&d, &out, 1_792_915_974).unwrap();
        assert_eq!(path, out.join("construct-logs-1792915974.txt"));
        let text = fs::read_to_string(path).unwrap();
        assert!(text.contains(concat!("App version: ", env!("CARGO_PKG_VERSION"))));
        let yesterday = text.find("yesterday").expect("the older day is in it");
        let today = text.find("today").expect("the newer day is in it");
        assert!(yesterday < today, "oldest first");
        assert!(text.contains("Log file 2/2: konstruct.2026-10-25.log"));
        assert!(!text.contains("not a log"));
    }

    /// The last lines are the newest file's, whole lines only, even when it is read from its
    /// middle.
    #[test]
    fn the_last_lines_are_the_newest_whole_lines() {
        let d = dir("recent");
        day(&d, "2026-10-24", "old\n");
        // 13-byte lines, so the read starts 10 bytes into one.
        let long: String = (0..10_000).map(|i| format!("line {i:07}\n")).collect();
        assert_ne!((long.len() as u64 - TAIL_BYTES) % 13, 0);
        day(&d, "2026-10-25", &long);
        assert_eq!(recent(&d, 2), "line 0009998\nline 0009999");
        let many = recent(&d, 100_000);
        assert!(!many.contains("old"));
        let first = many.lines().next().unwrap();
        assert!(
            first.starts_with("line ") && first.len() == 12,
            "{first:?} is whole"
        );
    }

    /// Clearing removes the older days and empties the one being written, which stays.
    #[test]
    fn clearing_keeps_the_file_being_written_and_empties_it() {
        let d = dir("clear");
        let old = day(&d, "2026-10-24", "old\n");
        let current = day(&d, "2026-10-25", "current\n");
        let mut writer = fs::OpenOptions::new().append(true).open(&current).unwrap();
        clear(&d).unwrap();
        assert!(!old.exists());
        assert_eq!(fs::read_to_string(&current).unwrap(), "");
        std::io::Write::write_all(&mut writer, b"after\n").unwrap();
        assert_eq!(fs::read_to_string(&current).unwrap(), "after\n");
        assert_eq!(size(&d), 6);
    }
}
