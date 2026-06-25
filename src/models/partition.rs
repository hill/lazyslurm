use serde::{Deserialize, Serialize};

/// A partition (queue) as reported by `sinfo -s`. Node counts come from the
/// `%F` field in allocated/idle/other/total form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partition {
    pub name: String,
    /// SLURM marks the default partition with a trailing `*`; we strip it and
    /// record it here instead.
    pub is_default: bool,
    pub availability: String,
    pub nodes_alloc: u32,
    pub nodes_idle: u32,
    pub nodes_other: u32,
    pub nodes_total: u32,
    pub time_limit: String,
}

impl Partition {
    pub fn is_up(&self) -> bool {
        self.availability.eq_ignore_ascii_case("up")
    }
}
