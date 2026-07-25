use serde::{Deserialize, Serialize};

/// One fairshare association row from `sshare`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FairShareEntry {
    pub account: String,
    pub user: String,
    pub raw_shares: Option<u64>,
    pub norm_shares: Option<f64>,
    pub raw_usage: Option<u64>,
    pub effectv_usage: Option<f64>,
    pub fair_share: Option<f64>,
}

/// Where a fairshare factor sits relative to the neutral 0.5 midpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FairShareBand {
    /// Below the midpoint: recent usage ran ahead of target, mild priority penalty.
    Penalised,
    /// Around the midpoint: on target.
    Neutral,
    /// Above the midpoint: under target, a priority boost.
    Boosted,
}

impl FairShareEntry {
    /// Band the fairshare factor, treating a window around 0.5 as neutral.
    pub fn band(&self) -> Option<FairShareBand> {
        self.fair_share.map(|f| {
            if f < 0.45 {
                FairShareBand::Penalised
            } else if f > 0.55 {
                FairShareBand::Boosted
            } else {
                FairShareBand::Neutral
            }
        })
    }
}
