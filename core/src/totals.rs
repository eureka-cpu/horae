use uuid::Uuid;

use crate::rounding;
use crate::types::RoundDir;

pub struct OrgRounding {
    pub increment_minutes: u32,
    pub dir: RoundDir,
}

pub struct EntryInput {
    pub project_id: Uuid,
    pub minutes: u32,
    pub billable: bool,
}

pub struct Totals {
    pub total_minutes: u32,
    pub billable_minutes: u32,
    pub non_billable_minutes: u32,
    /// Per-project total (rounded) minutes.
    pub by_project: Vec<(Uuid, u32)>,
}

/// Compute totals for a set of entries, applying org-level rounding to each.
pub fn compute_totals(entries: &[EntryInput], rounding: &OrgRounding) -> Totals {
    let mut total = 0u32;
    let mut billable = 0u32;
    let mut non_billable = 0u32;
    let mut by_project: std::collections::HashMap<Uuid, u32> = std::collections::HashMap::new();

    for e in entries {
        let rounded =
            rounding::round(e.minutes, rounding.increment_minutes, rounding.dir);
        total += rounded;
        if e.billable {
            billable += rounded;
        } else {
            non_billable += rounded;
        }
        *by_project.entry(e.project_id).or_default() += rounded;
    }

    Totals {
        total_minutes: total,
        billable_minutes: billable,
        non_billable_minutes: non_billable,
        by_project: by_project.into_iter().collect(),
    }
}
