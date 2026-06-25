use serde::{Deserialize, Serialize};

/// A partition from `sinfo -s`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partition {
    pub name: String,
    /// Default partition (sinfo marks it with a trailing `*`).
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
