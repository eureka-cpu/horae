use dioxus::prelude::*;

use crate::route::Route;
use crate::server_fns;

#[component]
pub fn Login() -> Element {
    let mut email = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut error_msg = use_signal(|| Option::<String>::None);
    let nav = use_navigator();

    let handle_login = move |_| {
        let email_val = email.read().clone();
        let pass_val = password.read().clone();
        spawn(async move {
            match server_fns::login(email_val, pass_val).await {
                Ok(_) => { nav.push(Route::Dashboard {}); }
                Err(e) => { error_msg.set(Some(e.to_string())); }
            }
        });
    };

    rsx! {
        div { class: "auth-container",
            div { class: "auth-card",
                div { class: "auth-logo", "Horae" }
                h1 { style: "font-size: 1.25rem; margin-bottom: 1.5rem; text-align: center;", "Sign in to your account" }

                if let Some(err) = error_msg.read().as_ref() {
                    div { class: "alert alert-danger", "{err}" }
                }

                div { class: "form-group",
                    label { class: "form-label", "Email" }
                    input {
                        class: "form-input",
                        r#type: "email",
                        placeholder: "you@example.com",
                        value: "{email}",
                        oninput: move |e| email.set(e.value()),
                    }
                }
                div { class: "form-group",
                    label { class: "form-label", "Password" }
                    input {
                        class: "form-input",
                        r#type: "password",
                        placeholder: "••••••••",
                        value: "{password}",
                        oninput: move |e| password.set(e.value()),
                    }
                }
                button {
                    class: "btn btn-primary w-full mt-4",
                    onclick: handle_login,
                    "Sign In"
                }
            }
        }
    }
}

#[component]
pub fn Register() -> Element {
    rsx! {
        div { class: "auth-container",
            div { class: "auth-card",
                div { class: "auth-logo", "Horae" }
                h1 { style: "font-size: 1.25rem; margin-bottom: 1.5rem; text-align: center;", "Create Account" }
                p { class: "text-muted text-sm", "Registration is currently admin-only. Please contact your administrator." }
            }
        }
    }
}
