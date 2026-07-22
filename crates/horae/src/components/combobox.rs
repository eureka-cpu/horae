use dioxus::prelude::*;

/// One option in a [`Combobox`]. `group` places it under a heading; consecutive
/// options sharing a group render under a single heading.
#[derive(Clone, PartialEq)]
pub struct ComboOption {
    pub value: String,
    pub label: String,
    pub group: Option<String>,
}

impl ComboOption {
    pub fn grouped(
        value: impl Into<String>,
        label: impl Into<String>,
        group: impl Into<String>,
    ) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            group: Some(group.into()),
        }
    }
}

enum Row {
    Header(String),
    Item(ComboOption),
}

/// A searchable select: a trigger showing the current selection, and a popover
/// with a search box, a "clear" reset row, and the (optionally grouped) options
/// filtered live. Emits the chosen value (empty string when the reset is picked).
/// Box/item styling comes from the shared `.menu` / `.menu-item` classes.
#[component]
pub fn Combobox(
    options: Vec<ComboOption>,
    /// Currently selected value; empty string means nothing selected.
    value: String,
    /// Shown on the trigger when nothing is selected.
    placeholder: String,
    /// Label of the reset row at the top of the list.
    #[props(default = "All".to_string())]
    all_label: String,
    #[props(default)] onselect: EventHandler<String>,
) -> Element {
    let mut open = use_signal(|| false);
    let mut search = use_signal(String::new);

    let label = options
        .iter()
        .find(|o| o.value == value)
        .map(|o| o.label.clone())
        .unwrap_or_else(|| placeholder.clone());

    let q = search().to_lowercase();
    let mut rows: Vec<Row> = Vec::new();
    let mut last_group: Option<String> = None;
    for o in options
        .iter()
        .filter(|o| q.is_empty() || o.label.to_lowercase().contains(&q))
    {
        if o.group != last_group {
            if let Some(g) = &o.group {
                rows.push(Row::Header(g.clone()));
            }
            last_group = o.group.clone();
        }
        rows.push(Row::Item(o.clone()));
    }

    rsx! {
        div { class: "menu-anchor",
            button {
                r#type: "button",
                class: "btn btn-secondary btn-sm",
                "aria-haspopup": "listbox",
                "aria-expanded": "{open}",
                onclick: move |_| {
                    let next = !open();
                    open.set(next);
                },
                "{label}"
                span { class: "ml-2 text-faint", "▾" }
            }
            if open() {
                div { class: "menu-overlay", onclick: move |_| open.set(false) }
                div { class: "menu combobox-pop", role: "listbox",
                    div { class: "mb-2",
                        input {
                            class: "form-input w-full",
                            r#type: "text",
                            placeholder: "Search…",
                            aria_label: "Search",
                            value: "{search}",
                            oninput: move |e| search.set(e.value()),
                        }
                    }
                    button {
                        r#type: "button",
                        class: if value.is_empty() { "menu-item selected" } else { "menu-item" },
                        onclick: move |_| {
                            onselect.call(String::new());
                            search.set(String::new());
                            open.set(false);
                        },
                        "{all_label}"
                    }
                    for row in rows {
                        match row {
                            Row::Header(g) => rsx! {
                                div { class: "menu-group", "{g}" }
                            },
                            Row::Item(o) => {
                                let val = o.value.clone();
                                let selected = o.value == value;
                                rsx! {
                                    button {
                                        r#type: "button",
                                        class: if selected { "menu-item selected" } else { "menu-item" },
                                        onclick: move |_| {
                                            onselect.call(val.clone());
                                            search.set(String::new());
                                            open.set(false);
                                        },
                                        "{o.label}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
