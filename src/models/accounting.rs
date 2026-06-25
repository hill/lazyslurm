use serde::{Deserialize, Serialize};

/// One finished job from `sacct`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcctEntry {
    pub job_id: String,
    pub name: String,
    pub state: String,
    /// `code:signal`, e.g. `0:0`.
    pub exit_code: String,
    pub elapsed: String,
    pub start: String,
    pub end: String,
}

impl AcctEntry {
    /// True when the job ended cleanly (`0:0` and COMPLETED).
    pub fn succeeded(&self) -> bool {
        self.exit_code == "0:0" && self.state.eq_ignore_ascii_case("COMPLETED")
    }
}

/// Full `sacct -j` detail for one job, for the History detail view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcctDetail {
    pub job_id: String,
    pub name: String,
    pub user: String,
    pub account: String,
    pub partition: String,
    pub node_list: String,
    pub alloc_cpus: String,
    pub req_mem: String,
    pub max_rss: Option<String>,
    pub total_cpu: String,
    pub state: String,
    pub exit_code: String,
    pub submit: String,
    pub start: String,
    pub end: String,
    pub elapsed: String,
    pub work_dir: String,
}
