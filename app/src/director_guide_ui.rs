//! Options bridge; the companion owns progress and never writes Toolkit settings.
use crate::{paths, AppWindow};
use slint::ComponentHandle;
use tf2_mirv_director::guide::{Request, Store};
fn store() -> Store {
    Store(paths::config_root().join("Director Guide"))
}
fn refresh(ui: &AppWindow) {
    match (store().preferences(), store().progress()) {
        (Ok(p), Ok(s)) => {
            ui.set_director_guide_enabled(p.enabled);
            ui.set_director_guide_can_resume(s.seen && !s.completed);
            let status = if p.revision != s.handled_revision && p.request != Request::None {
                if p.request == Request::Reset {
                    "Welcome will show on the next Director launch.".into()
                } else {
                    "Guide requested. It will open when Director and the demo are ready.".into()
                }
            } else if s.completed {
                "Guide completed. Start again any time.".into()
            } else if s.seen {
                format!(
                    "Saved at step {} of 30. Resume in Director or here.",
                    s.step.min(29) + 1
                )
            } else {
                "Shows automatically on your first Director session.".into()
            };
            ui.set_director_guide_status(status.into());
        }
        (Err(e), _) | (_, Err(e)) => {
            ui.set_director_guide_status(format!("Could not read guide state: {e}").into())
        }
    }
}
pub fn bind(ui: &AppWindow) {
    refresh(ui);
    let weak = ui.as_weak();
    ui.on_director_guide_action(move |action| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let result = (|| -> anyhow::Result<()> {
            let store = store();
            let mut prefs = store.preferences()?;
            match action {
                0 => {
                    prefs.enabled = !prefs.enabled;
                    prefs.request = Request::None;
                }
                1 => {
                    prefs.enabled = true;
                    prefs.request = Request::Start;
                }
                2 => {
                    prefs.enabled = true;
                    prefs.request = Request::Resume;
                }
                3 => {
                    prefs.enabled = true;
                    prefs.request = Request::Reset;
                }
                _ => return Ok(()),
            }
            store.save_preferences(&mut prefs)
        })();
        match result {
            Ok(()) => refresh(&ui),
            Err(e) => {
                ui.set_director_guide_status(format!("Could not save guide options: {e}").into())
            }
        }
    });
    let weak = ui.as_weak();
    ui.on_director_guide_tick(move || {
        if let Some(ui) = weak.upgrade() {
            refresh(&ui);
        }
    });
}
