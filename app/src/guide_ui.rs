//! UI controller for the opt-in, resumable workflow tour. Navigation never runs jobs.
use std::sync::Arc;
use parking_lot::Mutex;
use slint::ComponentHandle;
use crate::{guide, AppWindow, GuideFocus, State, Tf2Tooltips};

type SharedState = Arc<Mutex<State>>;

fn save(ui: &AppWindow, state: &SharedState) {
    let current = state.lock();
    ui.set_guide_can_resume(current.settings.onboarding.resumable());
    if let Err(error) = current.settings.save() {
        ui.set_options_save_status(format!("Could not save guide progress: {error}").into());
    }
}
fn working(ui: &AppWindow, state: &SharedState) -> bool {
    let current = state.lock();
    ui.get_busy() || ui.get_importing_demos() || current.recording_active || current.recovery_active
}

/// Also used by visual fixtures: metadata and geometry use the production UI.
pub fn show_step(ui: &AppWindow, index: i32) {
    let index = index.clamp(1, guide::MAX_STEP);
    let step = guide::step(index);
    ui.global::<GuideFocus>().set_available(false);
    ui.set_guide_step(index);
    ui.set_guide_ordinal(guide::ordinal(index));
    ui.set_guide_total(guide::ORDER.len() as i32);
    ui.global::<GuideFocus>().set_target(index);
    ui.set_guide_title(step.title.into());
    ui.set_guide_body(step.body.into());
    ui.set_guide_position(step.position);
    ui.set_guide_count(step.count);
    ui.set_guide_note("".into());
    ui.set_guide_can_next(!matches!(index, 11 | 12));
    ui.set_guide_next_label(if index == guide::LAST_STEP { "FINISH" } else { "NEXT" }.into());
    ui.set_guide_dialog(0);
    ui.set_candidate_filters_open(false);
    ui.set_compact_nav_open(false);
    ui.set_help_dialog_open(false);
    ui.set_selected_page(step.page);
    if step.page == 0 { ui.set_parse_step(step.section); }
    if step.page == 2 { ui.set_recording_settings_section(step.section); }
    ui.set_guide_open(true);
    ui.global::<GuideFocus>().set_active(true);
    ui.global::<Tf2Tooltips>().set_visible(false);
    encoding_copy(ui);
}
fn encoding_copy(ui: &AppWindow) {
    let index = ui.get_guide_step();
    if !matches!(index, 26 | 27) { return; }
    let format = ui.get_recording_format();
    let body = if format == "MP4 - Lossless" {
        "Lossless MP4 uses the built-in lossless preset and produces larger files. There are no additional encoder controls. Continue to Profile."
    } else if !guide::has_encoding_controls(&format) {
        "Image sequences save individual frames instead of an encoded video. There are no video encoder controls here. JPG quality is on Paths & Output. Continue to Profile."
    } else if format.starts_with("MOV") {
        "DNxHR is an editing format. Choose the profile supported by your editor; higher-quality profiles can create larger files. Continue with the default if you are unsure."
    } else if format.starts_with("AVI") {
        "Choose the AVI codec and its available pixel format. Lossless and raw formats can need substantial disk space. Keep the defaults unless your editing workflow needs another format."
    } else if format == "MP4 - Lossless" {
        "Lossless MP4 preserves more image detail and produces larger files. Check that your editor supports it. There are no Standard MP4 compatibility controls for this format."
    } else if index == 27 && ui.get_mp4_compatibility() != "Custom" {
        "Your compatibility preset manages encoding automatically. You do not need to change it. Choose Custom only if you want to adjust CRF, encoder and pixel format yourself."
    } else { guide::step(index).body };
    ui.set_guide_body(body.into());
}
fn go(ui: &AppWindow, state: &SharedState, index: i32) {
    show_step(ui, index);
    {
        let mut current = state.lock();
        current.settings.onboarding.seen_version = guide::VERSION;
        current.settings.onboarding.resume_step = index;
        current.settings.onboarding.completed = false;
    }
    save(ui, state);
    update_gate(ui, state);
}
fn start(ui: &AppWindow, state: &SharedState, resume: bool) {
    if working(ui, state) { ui.set_guide_dialog(3); return; }
    ui.set_guide_welcome_pending(false);
    ui.set_guide_queued(false);
    let saved = state.lock().settings.onboarding.resume_step;
    let index = if resume { guide::resume_at(saved, ui.get_imported_demo_count() > 0, ui.get_has_export(), ui.get_parse_options_ready()) } else { 1 };
    if index <= 11 { ui.set_parse_started(false); }
    go(ui, state, index);
}
fn update_gate(ui: &AppWindow, state: &SharedState) {
    let index = ui.get_guide_step();
    let has_demos = ui.get_imported_demo_count() > 0;
    let current = state.lock();
    let no_candidates = current.candidates.is_empty();
    let no_visible = current.visible.is_empty();
    let (can_next, note) = match index {
        1 if !has_demos => (false, "Add demos to continue, or load a previously parsed export using the gold button."),
        2 | 4 if !has_demos => (false, "Choose at least one demo first."),
        8 if !ui.get_parse_options_ready() => (false, "Choose a valid export location. Item schema is optional."),
        11 => (false, "Use the app's Parse Demos button. The guide never starts parsing for you."),
        12 if !ui.get_busy() => (false, "Parsing stopped or results could not be loaded. Check Application Logs, then go Back to retry."),
        12 => (false, "The guide will continue when parsing and analysis finish."),
        13 if no_candidates => (false, "This export has no candidates. Go Back to choose other demos, or pause the guide in Options."),
        16 if no_visible => (false, "No rows match. Clear the search or relax Filters before selecting a candidate."),
        16 if ui.get_selected_count() == 0 => (false, "Select a candidate row to continue. View Details is optional."),
        35 | 36 if ui.get_selected_count() != 1 => (true, "This action needs exactly one selected candidate."),
        37 if ui.get_selected_count() == 0 => (true, "Select one or more rows before recording."),
        _ => (true, ""),
    };
    ui.set_guide_can_next(can_next);
    ui.set_guide_note(note.into());
    ui.set_guide_next_label(if index == guide::LAST_STEP { "FINISH" } else if index == 11 || index == 12 { "WAITING" } else { "NEXT" }.into());
    encoding_copy(ui);
}

pub fn results_loaded(ui: &AppWindow, state: &SharedState) {
    // Only a successful result load advances the parse phase. Existing old exports
    // and a failed/cancelled parse must never count as new results.
    if ui.get_guide_open() && ui.get_guide_step() <= 12 {
        let dialog = ui.get_guide_dialog();
        go(ui, state, 13);
        ui.set_guide_dialog(dialog);
    } else if !ui.get_guide_open() {
        let advance = {
            let current = state.lock();
            current.settings.onboarding.resumable() && current.settings.onboarding.resume_step <= 12
        };
        if advance { state.lock().settings.onboarding.resume_step = 13; save(ui, state); }
    }
}

pub fn bind(ui: &AppWindow, state: &SharedState) {
    {
        let current = state.lock();
        ui.set_guide_welcome_pending(current.settings.onboarding.should_offer());
        ui.set_guide_can_resume(current.settings.onboarding.resumable());
    }
    let weak = ui.as_weak();
    let action_state = state.clone();
    ui.on_guide_action(move |action| {
        let Some(ui) = weak.upgrade() else { return };
        let state = &action_state;
        match action {
            0 => start(&ui, state, false),
            1 if ui.get_guide_can_next() => {
                if ui.get_guide_step() == guide::LAST_STEP {
                    { let mut current = state.lock(); current.settings.onboarding.completed = true; current.settings.onboarding.resume_step = 0; }
                    ui.set_guide_open(false); ui.set_guide_dialog(0); save(&ui, state);
                } else {
                    let index = ui.get_guide_step();
                    if index == 8 { ui.invoke_commit_export_location(); if !ui.get_parse_options_ready() { update_gate(&ui, state); return; } }
                    go(&ui, state, guide::next(index, &ui.get_recording_format()));
                }
            }
            2 if !ui.get_busy() => {
                let index = ui.get_guide_step();
                let previous = if index == 13 { 1 } else { guide::previous(index, &ui.get_recording_format(), ui.get_imported_demo_count() > 0) };
                if previous <= 11 { ui.set_parse_started(false); }
                go(&ui, state, previous);
            }
            3 if ui.get_guide_open() => ui.set_guide_dialog(2),
            4 => { if ui.get_guide_dialog() == 2 { ui.set_guide_dialog(0); } else { start(&ui, state, true); } }
            5 => {
                ui.set_guide_open(false); ui.set_guide_dialog(0);
                { let mut current = state.lock(); current.settings.onboarding.resume_step = 0; current.settings.onboarding.seen_version = guide::VERSION; }
                save(&ui, state);
            }
            6 => {
                state.lock().settings.onboarding.reset_welcome();
                ui.set_guide_welcome_pending(false); save(&ui, state);
                ui.set_options_save_status("Welcome reset. Restart the app to test it. Your other settings are unchanged.".into());
            }
            7 => {
                ui.set_guide_dialog(0); ui.set_guide_welcome_pending(false); ui.set_guide_queued(false);
                state.lock().settings.onboarding.seen_version = guide::VERSION; save(&ui, state);
            }
            8 => { ui.set_guide_dialog(0); ui.set_guide_queued(true); ui.set_options_save_status("Guide queued; it will start after the current job finishes.".into()); }
            10 => { ui.set_guide_open(false); ui.set_guide_dialog(0); save(&ui, state); }
            _ => {}
        }
    });
    let weak = ui.as_weak();
    let tick_state = state.clone();
    ui.on_guide_tick(move || {
        let Some(ui) = weak.upgrade() else { return };
        let state = &tick_state;
        if !working(&ui, state) && ui.get_guide_dialog() == 0 {
            if ui.get_guide_queued() { start(&ui, state, false); return; }
            if ui.get_guide_welcome_pending() && !ui.get_guide_open() {
                ui.set_guide_welcome_pending(false); ui.set_guide_dialog(1); return;
            }
        }
        if !ui.get_guide_open() || ui.get_guide_dialog() != 0 { return; }
        let index = ui.get_guide_step();
        let page = ui.get_selected_page();
        // Details and help are temporary detours; return to the same tour step.
        if page == 4 || ui.get_help_dialog_open() { return; }
        if page == 3 || page == 5 {
            ui.set_guide_open(false); save(&ui, state); return;
        }
        if page == 0 && ui.get_parse_started() && ui.get_busy() && index <= 12 && index != 12 {
            go(&ui, state, 12); return;
        }
        if (2..=12).contains(&index) && ui.get_imported_demo_count() == 0 && !ui.get_busy() {
            go(&ui, state, 1); return;
        }
        let metadata = guide::step(index);
        let section = if page == 0 { ui.get_parse_step() } else { ui.get_recording_settings_section() };
        if page != metadata.page || ((page == 0 || page == 2) && section != metadata.section) {
            let next = match page {
                0 => match section { 0 => 1, 1 => 5, _ => if ui.get_parse_started() { 12 } else { 9 } },
                1 => if metadata.page == 2 { 35 } else { 13 },
                2 => [19, 23, 26, 28, 49][section.clamp(0, 4) as usize],
                _ => index,
            };
            go(&ui, state, next); return;
        }
        update_gate(&ui, state);
    });
}
