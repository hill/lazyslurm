use serde::{Deserialize, Serialize};

/// A compute node from `sinfo`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub name: String,
    pub state: String,
    pub cpus_alloc: u32,
    pub cpus_idle: u32,
    pub cpus_other: u32,
    pub cpus_total: u32,
    pub memory_mb: Option<u64>,
    pub free_mem_mb: Option<u64>,
    /// Generic resources, e.g. `gpu:a100:4`.
    pub gres: Option<String>,
    pub partition: String,
}

impl Node {
    /// Fraction of cores allocated, 0.0 to 1.0.
    pub fn cpu_load(&self) -> f32 {
        if self.cpus_total == 0 {
            0.0
        } else {
            self.cpus_alloc as f32 / self.cpus_total as f32
        }
    }

    /// Down, drained, failed, or in maintenance.
    pub fn is_unavailable(&self) -> bool {
        let s = self.state.to_lowercase();
        s.contains("down")
            || s.contains("drain")
            || s.contains("fail")
            || s.contains("maint")
            || s.contains("inval")
    }
}
