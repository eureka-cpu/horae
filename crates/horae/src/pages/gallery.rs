use dioxus::prelude::*;

use crate::components::avatar::{Avatar, Chip};
use crate::components::badge::Badge;
use crate::components::button::{Button, IconButton, SplitButton};
use crate::components::card::{Card, MetricCard};
use crate::components::controls::{Checkbox, Radio, Segmented, Toggle};
use crate::components::form::{FormGroup, Input, Select, Textarea};
use crate::components::combobox::{ComboOption, Combobox};
use crate::components::menu::{Menu, MenuDivider, MenuItem};
use crate::components::nav::NavItem;
use crate::components::table::DataTable;
use crate::components::toast::Toast;

/// A living gallery of the Horae component kit, mirroring the design system's
/// "Components" sheet. Also serves as a smoke test that every component renders.
#[component]
pub fn Gallery() -> Element {
    let mut segment = use_signal(|| "Week".to_string());
    let mut billable = use_signal(|| true);
    let mut agreed = use_signal(|| false);
    let mut plan = use_signal(|| "Manager".to_string());
    let mut combo = use_signal(String::new);

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Component Library" }
            }

            // ── Buttons ──────────────────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Buttons" }
                div { class: "gallery-row",
                    Button { variant: "primary", "Primary" }
                    Button { variant: "solid", "Submit week" }
                    Button { variant: "secondary", "Secondary" }
                    Button { variant: "accent", "Send invoice" }
                    Button { variant: "danger", "Delete" }
                    Button { variant: "ghost", "Ghost" }
                }
                div { class: "gallery-row",
                    Button { variant: "primary", size: "sm", "Small" }
                    Button { variant: "primary", disabled: true, "Disabled" }
                    IconButton { label: "Start timer", "▶" }
                    SplitButton { label: "Generate PDF" }
                }
            }

            // ── Status pills ─────────────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Status pills" }
                div { class: "gallery-row",
                    Badge { variant: "success", "Approved" }
                    Badge { variant: "info", "Synced" }
                    Badge { variant: "warning", "Awaiting" }
                    Badge { variant: "danger", "Overdue" }
                    Badge { variant: "neutral", "Draft" }
                }
            }

            // ── Inputs & fields ──────────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Inputs & fields" }
                div { class: "gallery-row",
                    div { style: "min-width: 220px",
                        FormGroup { label: "Default", hint: "A short helper line.",
                            Input { placeholder: "casey@example.com" }
                        }
                    }
                    div { style: "min-width: 220px",
                        FormGroup { label: "Numeric",
                            Input { kind: "number", value: "128" }
                        }
                    }
                    div { style: "min-width: 220px",
                        FormGroup { label: "Read only",
                            Input { value: "INV-2026-0007", readonly: true }
                        }
                    }
                    div { style: "min-width: 220px",
                        FormGroup { label: "Disabled",
                            Input { placeholder: "Unavailable", disabled: true }
                        }
                    }
                    div { style: "min-width: 220px",
                        FormGroup { label: "Dropdown",
                            Select {
                                selected: plan(),
                                options: vec![
                                    ("Member".to_string(), "Member".to_string()),
                                    ("Manager".to_string(), "Manager".to_string()),
                                    ("Admin".to_string(), "Admin".to_string()),
                                ],
                                onchange: move |e: FormEvent| plan.set(e.value()),
                            }
                        }
                    }
                }
                div { style: "max-width: 460px",
                    FormGroup { label: "Notes",
                        Textarea { placeholder: "Kickoff call with Acme…" }
                    }
                }
            }

            // ── Toggles & segments ───────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Toggles & segments" }
                div { class: "gallery-row",
                    Segmented {
                        items: vec!["Day".to_string(), "Week".to_string(), "Calendar".to_string()],
                        active: segment(),
                        onselect: move |v| segment.set(v),
                    }
                    Toggle {
                        on: billable(),
                        label: "Billable",
                        onclick: move |_| billable.set(!billable()),
                    }
                }
                div { class: "gallery-row",
                    Checkbox {
                        checked: agreed(),
                        label: "I agree",
                        onclick: move |_| agreed.set(!agreed()),
                    }
                    Radio { selected: true, label: "Solid" }
                    Radio { selected: false, label: "Split" }
                }
            }

            // ── Dropdown menu ────────────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Dropdown menu" }
                div { class: "gallery-row",
                    Menu { label: "Actions",
                        MenuItem { onclick: move |_| {}, "Edit" }
                        MenuItem { selected: true, onclick: move |_| {}, "Pin" }
                        MenuDivider {}
                        MenuItem { onclick: move |_| {}, "Archive" }
                        MenuItem { danger: true, onclick: move |_| {}, "Delete" }
                        MenuItem { disabled: true, "Unavailable" }
                    }
                    Combobox {
                        options: vec![
                            ComboOption::grouped("1", "Numtide", "Active clients"),
                            ComboOption::grouped("2", "Accur8 Software", "Active clients"),
                            ComboOption::grouped("3", "Golem SBB", "Archived clients"),
                        ],
                        value: combo(),
                        placeholder: "Filter by client",
                        all_label: "All clients",
                        onselect: move |v| combo.set(v),
                    }
                }
            }

            // ── Nav item ─────────────────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Nav item" }
                div { style: "max-width: 240px; display: flex; flex-direction: column; gap: 4px",
                    NavItem { icon: "◷", label: "Timesheet", active: true }
                    NavItem { icon: "▤", label: "Approvals" }
                    NavItem { icon: "◑", label: "Reports" }
                }
            }

            // ── Avatar & chips ───────────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Avatar & chips" }
                div { class: "gallery-row",
                    Avatar { initials: "LE", size: "sm" }
                    Avatar { initials: "LE" }
                    Avatar { initials: "LE", size: "lg" }
                    Chip { label: "Lars Ericsson" }
                    Chip { label: "Casey Rivera" }
                    Chip { label: "Time & Materials", plain: true }
                    Chip { label: "Manager", plain: true }
                }
            }

            // ── Cards ────────────────────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Cards" }
                div { class: "gallery-row",
                    MetricCard { label: "Hours this week", value: "128.5", delta: "+12%", direction: "up" }
                    MetricCard { label: "Unbilled", value: "$8,240", delta: "-3%", direction: "down" }
                    Card { title: "Frontend engineering",
                        p { class: "text-muted text-sm", "Time & materials · EUR" }
                    }
                }
            }

            // ── Table & rows ─────────────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Table & rows" }
                DataTable {
                    table {
                        thead {
                            tr {
                                th { "Teammate" }
                                th { "Project" }
                                th { class: "text-right", "Hours" }
                                th { "Status" }
                                th { class: "text-right", "Action" }
                            }
                        }
                        tbody {
                            tr {
                                td { Chip { label: "Lars Ericsson" } }
                                td { "Acme redesign" }
                                td { class: "text-mono text-right", "12.0" }
                                td { Badge { variant: "success", "Approved" } }
                                td { class: "text-right",
                                    Button { variant: "ghost", size: "sm", "Reopen" }
                                }
                            }
                            tr {
                                td { Chip { label: "Casey Rivera" } }
                                td { "Globex API" }
                                td { class: "text-mono text-right", "6.5" }
                                td { Badge { variant: "warning", "Awaiting" } }
                                td { class: "text-right",
                                    Button { variant: "primary", size: "sm", "Approve" }
                                }
                            }
                        }
                    }
                }
            }

            // ── Toast & empty state ──────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Toast & empty state" }
                div { class: "gallery-row",
                    Toast { message: "Invoice sent to Acme.", variant: "success", icon: "✓" }
                    Toast { message: "Timer still running.", variant: "warning", icon: "⏱" }
                }
                div { class: "empty-state", style: "max-width: 420px",
                    div { class: "empty-state-icon", "🗂" }
                    div { class: "empty-state-title", "No time entries yet" }
                    p { class: "text-muted text-sm", "Start a timer or add an entry to see it here." }
                }
            }
        }
    }
}
