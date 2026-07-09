use crate::types::{EntryState, OrgRole};

/// Returns true if the transition from `from` to `to` is allowed for `role`.
pub fn can_transition(from: EntryState, to: EntryState, role: OrgRole) -> bool {
    use EntryState::*;
    use OrgRole::*;
    match (from, to) {
        // Any member can submit open entries
        (Open, Submitted) => true,
        // Members can reopen (pull back) their own submission
        (Submitted, Open) => true,
        // Managers and admins can approve
        (Submitted, Approved) => matches!(role, Manager | Admin),
        // Managers and admins can reject (reopen from submitted)
        (Approved, Open) => matches!(role, Manager | Admin),
        // Admins can mark approved entries as invoiced
        (Approved, Invoiced) => matches!(role, Admin),
        // No other transitions allowed
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use EntryState::*;
    use OrgRole::*;

    #[test]
    fn member_submit_and_reopen() {
        assert!(can_transition(Open, Submitted, Member));
        assert!(can_transition(Submitted, Open, Member));
    }

    #[test]
    fn only_manager_can_approve() {
        assert!(!can_transition(Submitted, Approved, Member));
        assert!(can_transition(Submitted, Approved, Manager));
        assert!(can_transition(Submitted, Approved, Admin));
    }

    #[test]
    fn invoiced_is_final() {
        assert!(!can_transition(Invoiced, Open, Admin));
        assert!(!can_transition(Invoiced, Submitted, Admin));
    }
}
