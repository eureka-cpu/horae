# User Administration Requirements Quality Checklist

**Purpose**: Validate completeness and clarity of US4 (Administer users & access) requirements
**Created**: 2026-07-11
**Feature**: [spec.md](../spec.md) — User Story 4, FR-001, FR-002

## Requirement Completeness

- [ ] CHK001 - Are all CRUD operations for user accounts specified (create, read/list, update role, deactivate)? [Completeness, Spec §FR-002]
- [ ] CHK002 - Are requirements for listing users defined — including whether inactive users should be visible to admins? [Gap]
- [ ] CHK003 - Is the email uniqueness constraint documented in the spec or only in the contract? [Completeness, Spec §FR-002]
- [ ] CHK004 - Are requirements for user reactivation (re-enabling a deactivated account) specified? [Gap]
- [ ] CHK005 - Is the CLI user management surface (user create, user list) specified alongside the server function surface? [Completeness, contracts/cli.md]

## Requirement Clarity

- [ ] CHK006 - Is "deactivated users MUST be unable to sign in" specific about all auth paths (OIDC, dev login, existing sessions)? [Clarity, Spec §FR-002]
- [ ] CHK007 - Is "restricted by role" quantified with a role-permission matrix mapping each action to its required role? [Clarity, Spec §FR-001]
- [ ] CHK008 - Is "historical entries remain intact" defined — which specific records must survive user deactivation? [Clarity, Edge Cases]
- [ ] CHK009 - Is the meaning of "the action is not available to them" (acceptance scenario 2) precise — does it mean hidden UI, an error response, or both? [Ambiguity, Spec §US4-AS2]

## Requirement Consistency

- [ ] CHK010 - Does the role hierarchy (member < manager < admin) consistently map to gating across FR-001, FR-002, FR-008, FR-009, FR-010, FR-012? [Consistency]
- [ ] CHK011 - Is the `list_users` access level consistent between the contract (currently member, planned admin) and the spec (FR-002 implies admin)? [Conflict, contracts/server-functions.md]
- [ ] CHK012 - Are the CLI `user create` and server function `create_user` requirements consistent in parameters and behavior? [Consistency, contracts/cli.md vs contracts/server-functions.md]

## Acceptance Criteria Quality

- [ ] CHK013 - Can "limited to the permissions of their role" (acceptance scenario 1) be objectively verified without an exhaustive role-permission mapping? [Measurability, Spec §US4-AS1]
- [ ] CHK014 - Can "the action is not available" (acceptance scenario 2) be objectively tested — are the specific actions a member should be denied enumerated? [Measurability, Spec §US4-AS2]

## Scenario Coverage

- [ ] CHK015 - Is the scenario for changing a user's role after initial assignment addressed in acceptance scenarios? [Coverage, Gap]
- [ ] CHK016 - Is the scenario for an admin creating another admin account covered? [Coverage, Gap]
- [ ] CHK017 - Are error/exception flows defined for user creation failures (invalid email, duplicate, missing fields)? [Coverage, Exception Flow]

## Edge Case Coverage

- [ ] CHK018 - Is the "last admin" scenario addressed — preventing demotion or deactivation of the sole administrator? [Edge Case, Gap]
- [ ] CHK019 - Is admin self-deactivation or self-demotion addressed? [Edge Case, Gap]
- [ ] CHK020 - Is the behavior specified when deactivating a user who has a running timer? [Edge Case, Gap]
- [ ] CHK021 - Is the behavior specified for deactivating a user who has draft (un-sent) invoices they created? [Edge Case, Gap]

## Non-Functional Requirements

- [ ] CHK022 - Are there performance or scale requirements for the users list (e.g., at SC-005's 50+ users)? [Gap, NFR]
- [ ] CHK023 - Are audit logging requirements defined for user management actions (create, role change, deactivation)? [Gap, NFR]
- [ ] CHK024 - Are accessibility requirements specified for the admin users UI? [Gap, NFR]

## Dependencies & Assumptions

- [ ] CHK025 - Is the dependency on the auth system (OIDC + dev login) for enforcing deactivation documented? [Dependency]
- [ ] CHK026 - Is the assumption that user creation migrates from CLI-only to server-function-based documented? [Assumption]

## Notes

- Focus: User administration, access control, role gating
- Depth: Standard
- Audience: Reviewer (PR)
- Generated for US4 tasks T033–T036
