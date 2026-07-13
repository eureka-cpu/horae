# Plugin System Requirements Quality Checklist

**Purpose**: Validate completeness and clarity of US5 (Extend with plugins) requirements
**Created**: 2026-07-13
**Feature**: [spec.md](../spec.md) — User Story 5, FR-018..FR-022, SC-006

## Requirement Completeness

- [ ] CHK001 - Are all five business events explicitly enumerated with their trigger conditions and payload schemas? [Completeness, Spec §FR-019]
- [ ] CHK002 - Are the four host functions (log, db_query, http_post, config_get) fully specified with input/output types and error behavior? [Completeness, Spec §FR-020]
- [ ] CHK003 - Are requirements for the plugin manifest schema (`plugin.toml`) complete — are all fields, their types, and validation rules documented? [Completeness, contracts/plugin-interface.md]
- [ ] CHK004 - Are requirements for plugin installation and directory structure defined (`{dataDir}/plugins/` layout)? [Completeness, Spec §FR-018]
- [ ] CHK005 - Are dashboard widget requirements complete — is the return schema, rendering location, and sanitization behavior specified? [Completeness, Spec §FR-022]
- [ ] CHK006 - Are requirements for plugin configuration storage and retrieval defined (how operators set per-plugin config)? [Gap]
- [ ] CHK007 - Are requirements defined for what happens when zero plugins are installed? [Gap, Edge Case]

## Requirement Clarity

- [ ] CHK008 - Is "within 1 second" (SC-006) clearly defined — does it mean dispatch latency, total plugin execution time, or end-to-end from trigger to subscriber completion? [Ambiguity, Spec §SC-006]
- [ ] CHK009 - Is the timeout duration for plugin invocations quantified with a specific value or configurable range? [Clarity, contracts/plugin-interface.md]
- [ ] CHK010 - Is "read-only" for `horae_db_query` precisely defined — is it enforced via a read-only transaction, SQL parsing, or a restricted DB role? [Clarity, Spec §FR-020]
- [ ] CHK011 - Is "sanitized" HTML for dashboard widgets defined with specific rules (which tags/attributes are allowed/denied)? [Ambiguity, contracts/plugin-interface.md]
- [ ] CHK012 - Is "designated plugin area" on the dashboard quantified with specific layout/positioning requirements? [Ambiguity, Spec §FR-022]
- [ ] CHK013 - Is "operator-trusted but sandboxed" clearly delineated — what trust boundary does the operator cross vs what the sandbox enforces regardless? [Clarity, Spec Assumptions]

## Requirement Consistency

- [ ] CHK014 - Are the event names consistent between the spec (FR-019), the contract's event catalog, and the tasks (T039/T043)? [Consistency]
- [ ] CHK015 - Are the host function capabilities consistent between FR-020's "explicitly granted capabilities" list and the contract's four host functions? [Consistency, Spec §FR-020 vs contracts/plugin-interface.md]
- [ ] CHK016 - Is the dispatch-after-commit guarantee (contract §Sandbox) consistent with the "never blocks the core action" requirement (FR-021)? [Consistency]
- [ ] CHK017 - Are plugin rejection criteria consistent between the spec edge case ("malformed, unsupported, or malicious plugin is rejected at load time") and the contract's manifest validation rules? [Consistency]

## Acceptance Criteria Quality

- [ ] CHK018 - Can "the plugin is loaded and registered for those events" (acceptance scenario 1) be objectively verified — what constitutes "registered"? [Measurability, Spec §US5-AS1]
- [ ] CHK019 - Can "the core action still completes" (acceptance scenario 3) be measured — is there a defined threshold for acceptable delay introduced by plugin dispatch? [Measurability, Spec §US5-AS3]
- [ ] CHK020 - Can "the widget's content is displayed in a designated plugin area" (acceptance scenario 4) be objectively verified without UI layout specifications? [Measurability, Spec §US5-AS4]

## Scenario Coverage

- [ ] CHK021 - Are requirements defined for plugin hot-reload or runtime enable/disable without server restart? [Coverage, Gap]
- [ ] CHK022 - Are requirements defined for multiple plugins subscribing to the same event — ordering, concurrency, independence? [Coverage, contracts/plugin-interface.md]
- [ ] CHK023 - Are requirements specified for plugin versioning and upgrade scenarios (replacing a .wasm with a newer version)? [Coverage, Gap]
- [ ] CHK024 - Are requirements defined for the `user_logged_in` event's `method` field — what values are valid (oidc, dev, etc.)? [Coverage, contracts/plugin-interface.md]
- [ ] CHK025 - Are requirements specified for plugin output size limits (widget body, http_post response, db_query result set)? [Coverage, Gap]

## Edge Case Coverage

- [ ] CHK026 - Is the behavior specified when a plugin's declared hook name doesn't match any exported WASM function? [Edge Case, contracts/plugin-interface.md]
- [ ] CHK027 - Is the behavior defined when `horae_http_post` targets an unreachable or slow external URL? [Edge Case, Gap]
- [ ] CHK028 - Is the behavior specified when `horae_db_query` receives invalid SQL or returns an excessively large result? [Edge Case, Gap]
- [ ] CHK029 - Is the behavior defined when multiple plugins contribute dashboard widgets — ordering, layout overflow, empty widgets? [Edge Case, Gap]
- [ ] CHK030 - Are requirements for handling a corrupt or non-WASM file in the plugins directory specified? [Edge Case, Spec Edge Cases]
- [ ] CHK031 - Is the behavior defined when a plugin panics mid-execution of a host function call? [Edge Case, Gap]

## Non-Functional Requirements

- [ ] CHK032 - Are memory limits for individual plugin WASM instances specified? [Gap, NFR]
- [ ] CHK033 - Are requirements defined for the performance impact of plugin dispatch on core operation latency? [Gap, NFR]
- [ ] CHK034 - Are logging/observability requirements specified for plugin lifecycle events (load, invoke, fail, timeout)? [Gap, NFR]
- [ ] CHK035 - Are requirements for network policy or outbound request restrictions on `horae_http_post` defined beyond "operator-configured"? [Gap, NFR]

## Dependencies & Assumptions

- [ ] CHK036 - Is the dependency on the `extism` runtime documented with version constraints and platform support scope? [Dependency]
- [ ] CHK037 - Is the assumption that plugins are "operator-trusted" reconciled with the sandbox enforcement requirements — what attack vectors are in scope vs out of scope? [Assumption, Spec Assumptions]
- [ ] CHK038 - Is the dependency between plugin dispatch (T043) and the server functions that emit events (T011, T026, T028) documented? [Dependency, tasks.md]

## Notes

- Focus: Plugin sandbox, event dispatch, failure isolation, host function interface
- Depth: Standard
- Audience: Reviewer (PR)
- Generated for US5 tasks T037–T045
