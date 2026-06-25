use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use crate::models::Job;
use crate::slurm::SlurmParser;

/// Size of the window read from the end of a log file.
pub const TAIL_BYTES: u64 = 256 * 1024;

pub enum LogRead {
    Lines { path: String, text: String },
    Empty(String),
    Missing(Vec<String>),
}

/// Tail the last `max_bytes`. A torn first line is dropped when windowed.
pub fn tail_file(path: &str, max_bytes: u64) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    if len == 0 {
        return Some(String::new());
    }

    let window = len.min(max_bytes);
    file.seek(SeekFrom::End(-(window as i64))).ok()?;

    let mut buf = Vec::with_capacity(window as usize);
    file.take(window).read_to_end(&mut buf).ok()?;

    let text = String::from_utf8_lossy(&buf).into_owned();

    // Seeked mid-line: drop up to the first newline.
    if window < len
        && let Some(nl) = text.find('\n')
    {
        return Some(text[nl + 1..].to_string());
    }

    Some(text)
}

/// Resolve a job's candidate log paths and tail the first one that reads.
pub fn read_tail_for_job(job: &Job, max_bytes: u64) -> LogRead {
    read_tail_for_paths(SlurmParser::get_job_log_paths(job), max_bytes)
}

/// Tail the first of `paths` that reads.
pub fn read_tail_for_paths(paths: Vec<String>, max_bytes: u64) -> LogRead {
    for path in &paths {
        match tail_file(path, max_bytes) {
            Some(text) if text.is_empty() => return LogRead::Empty(path.clone()),
            Some(text) => {
                return LogRead::Lines {
                    path: path.clone(),
                    text,
                };
            }
            None => continue,
        }
    }

    LogRead::Missing(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("lazyslurm_test_{name}"))
    }

    #[test]
    fn tail_drops_partial_first_line_when_windowed() {
        let path = temp_path("windowed.log");
        let mut f = File::create(&path).unwrap();
        writeln!(f, "AAAAAAAAAA").unwrap(); // torn by a small window
        writeln!(f, "keep me").unwrap();
        f.flush().unwrap();

        let out = tail_file(path.to_str().unwrap(), 12).unwrap();
        assert!(!out.contains('A'), "partial first line should be dropped");
        assert!(out.contains("keep me"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn tail_returns_whole_small_file() {
        let path = temp_path("small.log");
        std::fs::write(&path, "line one\nline two\n").unwrap();

        let out = tail_file(path.to_str().unwrap(), TAIL_BYTES).unwrap();
        assert_eq!(out, "line one\nline two\n");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn tail_handles_empty_file() {
        let path = temp_path("empty.log");
        std::fs::write(&path, "").unwrap();
        assert_eq!(
            tail_file(path.to_str().unwrap(), TAIL_BYTES),
            Some(String::new())
        );
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn tail_missing_file_is_none() {
        assert_eq!(tail_file("/no/such/lazyslurm/file.log", TAIL_BYTES), None);
    }
}
