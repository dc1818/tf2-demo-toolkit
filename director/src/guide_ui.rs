//! Tour navigation deliberately has no access to the Director command queue.
use super::*;
use crate::guide::{self, Preferences, Progress, Store};

struct Tour {
    store: Store,
    progress: Progress,
    active: bool,
    previous_panel: bool,
    previous_interaction: bool,
}
impl Tour {
    fn persist(&self, window: &DirectorGuideWindow) {
        if let Err(e) = self.store.save_progress(&self.progress) {
            window.set_note(format!("Could not save guide progress: {e}").into());
        }
    }
    fn display(
        &self,
        window: &DirectorGuideWindow,
        strip: &DirectorStripWindow,
        card: &DirectorCardWindow,
        session: &DirectorSession,
    ) {
        let index = self.progress.step.min(guide::STEPS.len() - 1);
        let step = &guide::STEPS[index];
        window.set_step(index as i32);
        window.set_heading(step.title.into());
        window.set_body(guide::body(index, &session.shortcuts).into());
        window.set_example((16..=20).contains(&index) && !card.get_has_selected_keyframe());
        let target = if !step.shortcut.is_empty() && !matches!(index, 1 | 22 | 27 | 28) {
            session
                .shortcuts
                .iter()
                .position(|s| s.id == step.shortcut)
                .map(|n| 1000 + n as i32)
                .unwrap_or(0)
        } else if index == 0 || (16..=20).contains(&index) && !card.get_has_selected_keyframe() {
            0
        } else {
            index as i32 + 1
        };
        let focus = card.global::<DirectorGuideFocus>();
        focus.set_target(target);
        focus.set_pane(0);
        strip.global::<DirectorGuideFocus>().set_target(target);
        strip.global::<DirectorGuideFocus>().set_pane(0);
    }
    fn start(
        &mut self,
        step: usize,
        prefs: &Preferences,
        window: &DirectorGuideWindow,
        strip: &DirectorStripWindow,
        card: &DirectorCardWindow,
        panel: &Cell<bool>,
        interaction: &Cell<bool>,
        session: &DirectorSession,
    ) -> Result<()> {
        if !self.active {
            self.previous_panel = panel.get();
            self.previous_interaction = interaction.get();
        }
        set_panel_visibility(strip, card, panel, true);
        if !interaction.get() {
            toggle_overlay_interaction(strip, card, interaction);
        }
        self.progress.step = step;
        self.display(window, strip, card, session);
        window.show()?;
        let _ = window.window().with_winit_window(|native| {
            force_topmost(native);
            native.focus_window();
        });
        self.active = true;
        self.progress.seen = true;
        self.progress.completed = false;
        self.progress.handled_revision = prefs.revision;
        card.set_guide_active(true);
        card.global::<DirectorGuideFocus>().set_active(true);
        strip.global::<DirectorGuideFocus>().set_active(true);
        self.persist(window);
        Ok(())
    }
    fn pause(
        &mut self,
        window: &DirectorGuideWindow,
        strip: &DirectorStripWindow,
        card: &DirectorCardWindow,
        panel: &Cell<bool>,
        interaction: &Cell<bool>,
        restore: bool,
    ) {
        self.active = false;
        let _ = window.hide();
        card.set_guide_active(false);
        card.global::<DirectorGuideFocus>().set_active(false);
        strip.global::<DirectorGuideFocus>().set_active(false);
        if restore {
            if interaction.get() != self.previous_interaction {
                toggle_overlay_interaction(strip, card, interaction);
            }
            set_panel_visibility(strip, card, panel, self.previous_panel);
        }
        self.persist(window);
    }
}

/// Retain the returned handles for the lifetime of the event loop.
pub(super) fn bind(
    session: Rc<DirectorSession>,
    strip: &DirectorStripWindow,
    card: &DirectorCardWindow,
    panel: Rc<Cell<bool>>,
    interaction: Rc<Cell<bool>>,
    ready: Rc<Cell<bool>>,
    monitor: Rc<Cell<Option<MonitorGeometry>>>,
    startup_revision: u64,
) -> Result<Option<(DirectorGuideWindow, Timer)>> {
    let Some(directory) = session.guide_directory.clone() else {
        return Ok(None);
    };
    let store = Store(directory);
    store.preferences()?;
    let window = DirectorGuideWindow::new()?;
    let tour = Rc::new(RefCell::new(Tour {
        progress: store.progress()?,
        store,
        active: false,
        previous_panel: false,
        previous_interaction: false,
    }));
    {
        let (weak_window, weak_strip, weak_card) =
            (window.as_weak(), strip.as_weak(), card.as_weak());
        let (tour, session, panel, interaction) = (
            tour.clone(),
            session.clone(),
            panel.clone(),
            interaction.clone(),
        );
        window.on_action(move |action| {
            let (Some(window), Some(strip), Some(card)) = (
                weak_window.upgrade(),
                weak_strip.upgrade(),
                weak_card.upgrade(),
            ) else {
                return;
            };
            let mut tour = tour.borrow_mut();
            if !tour.active {
                return;
            }
            match action {
                1 => tour.progress.step = tour.progress.step.saturating_sub(1),
                2 if tour.progress.step + 1 < guide::STEPS.len() => tour.progress.step += 1,
                2 | 4 => {
                    tour.progress.completed = true;
                    tour.pause(&window, &strip, &card, &panel, &interaction, true);
                    return;
                }
                _ => {
                    tour.pause(&window, &strip, &card, &panel, &interaction, true);
                    return;
                }
            }
            window.set_note("".into());
            tour.display(&window, &strip, &card, &session);
            tour.persist(&window);
        });
    }
    {
        let (weak_window, weak_strip, weak_card) =
            (window.as_weak(), strip.as_weak(), card.as_weak());
        let (tour, session, panel, interaction, ready) = (
            tour.clone(),
            session.clone(),
            panel.clone(),
            interaction.clone(),
            ready.clone(),
        );
        card.on_guide_requested(move || {
            let (Some(window), Some(strip), Some(card)) = (
                weak_window.upgrade(),
                weak_strip.upgrade(),
                weak_card.upgrade(),
            ) else {
                return;
            };
            let mut tour = tour.borrow_mut();
            if tour.active {
                tour.pause(&window, &strip, &card, &panel, &interaction, true);
                return;
            }
            if !ready.get() {
                return;
            }
            match tour.store.preferences() {
                Ok(p) if p.enabled => {
                    let step = if tour.progress.completed {
                        0
                    } else {
                        tour.progress.step
                    };
                    if let Err(e) = tour.start(
                        step,
                        &p,
                        &window,
                        &strip,
                        &card,
                        &panel,
                        &interaction,
                        &session,
                    ) {
                        card.set_command_status(format!("Guide: {e}").into());
                    }
                }
                Ok(_) => card.set_command_status(
                    "Enable the MIRV Director guide in Toolkit Options.".into(),
                ),
                Err(e) => card.set_command_status(format!("Guide preferences: {e}").into()),
            }
        });
    }
    let weak = window.as_weak();
    window.window().on_close_requested(move || {
        if let Some(window) = weak.upgrade() {
            window.invoke_action(3);
        }
        slint::CloseRequestResponse::HideWindow
    });
    let timer = Timer::default();
    let (weak_window, weak_strip, weak_card) = (window.as_weak(), strip.as_weak(), card.as_weak());
    timer.start(TimerMode::Repeated, Duration::from_millis(250), move || {
        let (Some(window), Some(strip), Some(card)) = (
            weak_window.upgrade(),
            weak_strip.upgrade(),
            weak_card.upgrade(),
        ) else {
            return;
        };
        let mut tour = tour.borrow_mut();
        let prefs = match tour.store.preferences() {
            Ok(p) => p,
            Err(e) => {
                window.set_note(format!("Guide preferences: {e}").into());
                return;
            }
        };
        if tour.active && (!ready.get() || !prefs.enabled || !panel.get() || !interaction.get()) {
            // Respect a user's new focus/panel choice. A reload or Options disable restores prior input.
            let restore = panel.get() && interaction.get();
            tour.pause(&window, &strip, &card, &panel, &interaction, restore);
        }
        if ready.get() {
            if let Some(step) =
                guide::requested_step(&prefs, &tour.progress, prefs.revision == startup_revision)
            {
                if let Err(e) = tour.start(
                    step,
                    &prefs,
                    &window,
                    &strip,
                    &card,
                    &panel,
                    &interaction,
                    &session,
                ) {
                    card.set_command_status(format!("Guide: {e}").into());
                }
            }
        }
        if tour.active {
            // Keep conditional examples truthful without touching actual selected-keyframe fields.
            window.set_example(
                (16..=20).contains(&tour.progress.step) && !card.get_has_selected_keyframe(),
            );
            place(&window, &strip, &card, monitor.get());
            let _ = window.window().with_winit_window(force_topmost);
        }
    });
    Ok(Some((window, timer)))
}
fn place(
    window: &DirectorGuideWindow,
    strip: &DirectorStripWindow,
    card: &DirectorCardWindow,
    geometry: Option<MonitorGeometry>,
) {
    let Some(g) = geometry else {
        return;
    };
    let available_w = g.size.width as f64 / g.scale;
    let available_h = g.size.height as f64 / g.scale;
    let width = 390f64.min(available_w);
    let height = (if window.get_example() { 400f64 } else { 320f64 }).min(available_h);
    let focus = if matches!(window.get_step(), 2 | 3 | 15) {
        strip.global::<DirectorGuideFocus>()
    } else {
        card.global::<DirectorGuideFocus>()
    };
    let (anchor_x, anchor_y) = if focus.get_pane() == 1 {
        let pos = strip.window().position();
        (
            (pos.x - g.position.x) as f64 / g.scale + focus.get_left() as f64,
            (pos.y - g.position.y) as f64 / g.scale
                + focus.get_top() as f64
                + focus.get_target_height() as f64
                + 8.,
        )
    } else {
        let pos = card.window().position();
        (
            (pos.x - g.position.x) as f64 / g.scale - width - 12.,
            (pos.y - g.position.y) as f64 / g.scale + focus.get_top() as f64,
        )
    };
    let (x, y) = clamp_position(anchor_x, anchor_y, width, height, available_w, available_h);
    let _ = window.window().with_winit_window(|native| {
        let _ = native.request_inner_size(winit::dpi::LogicalSize::new(width, height));
        native.set_outer_position(winit::dpi::PhysicalPosition::new(
            g.position.x + (x * g.scale) as i32,
            g.position.y + (y * g.scale) as i32,
        ));
    });
}
fn clamp_position(x: f64, y: f64, w: f64, h: f64, screen_w: f64, screen_h: f64) -> (f64, f64) {
    (
        x.max(0.).min((screen_w - w).max(0.)),
        y.max(0.).min((screen_h - h).max(0.)),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guide_stays_inside_small_and_offset_monitors() {
        assert_eq!(
            clamp_position(-100., 600., 390., 320., 800., 480.),
            (0., 160.)
        );
        assert_eq!(
            clamp_position(900., -200., 390., 320., 1920., 1080.),
            (900., 0.)
        );
    }
}

#[cfg(test)]
mod rendered_tests {
    use super::*;
    use slint::platform::{
        software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
        Platform, WindowAdapter,
    };
    struct Backend {
        windows: Rc<RefCell<Vec<Rc<MinimalSoftwareWindow>>>>,
        clock: Rc<Cell<u64>>,
    }
    impl Platform for Backend {
        fn create_window_adapter(
            &self,
        ) -> std::result::Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
            let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
            self.windows.borrow_mut().push(window.clone());
            Ok(window)
        }
        fn duration_since_start(&self) -> Duration {
            Duration::from_millis(self.clock.get())
        }
    }
    fn pump(clock: &Cell<u64>) {
        for _ in 0..6 {
            clock.set(clock.get() + 100);
            slint::platform::update_timers_and_animations();
        }
    }
    fn render(window: &Rc<MinimalSoftwareWindow>, name: &str, width: u32, height: u32) {
        window.set_size(slint::PhysicalSize::new(width, height));
        window.request_redraw();
        let mut pixels = vec![slint::Rgb8Pixel::default(); (width * height) as usize];
        window.draw_if_needed(|renderer| {
            renderer.render(&mut pixels, width as usize);
        });
        assert!(pixels.iter().any(|p| p.r != 0 || p.g != 0 || p.b != 0));
        if let Ok(dir) = std::env::var("TF2_TOOLKIT_GUIDE_SNAPSHOTS") {
            fs::create_dir_all(&dir).unwrap();
            let mut file = File::create(Path::new(&dir).join(format!("{name}.ppm"))).unwrap();
            write!(file, "P6\n{width} {height}\n255\n").unwrap();
            for p in pixels {
                file.write_all(&[p.r, p.g, p.b]).unwrap();
            }
        }
    }
    slint::slint! {
        import { OptionsPage } from "../app/ui/options.slint";
        component OptionsPreview inherits Window {
            width: 1000px; height: 600px;
            background: #181918;
            in property <bool> compact;
            OptionsPage { width: parent.width; height: parent.height; compact: root.compact;
                director-guide-enabled: true; director-guide-status: "Shows automatically on your first Director session.";
            }
        }
    }
    #[test]
    fn rendered_tour_first_use_navigation_and_options() {
        let windows = Rc::new(RefCell::new(Vec::new()));
        let clock = Rc::new(Cell::new(0));
        slint::platform::set_platform(Box::new(Backend {
            windows: windows.clone(),
            clock: clock.clone(),
        }))
        .unwrap();
        let store = Store(
            std::env::temp_dir().join(format!("toolkit-rendered-guide-{}", std::process::id())),
        );
        let _ = fs::remove_dir_all(&store.0);
        let strip = DirectorStripWindow::new().unwrap();
        let card = DirectorCardWindow::new().unwrap();
        strip.show().unwrap();
        card.show().unwrap();
        windows.borrow()[0].set_size(slint::PhysicalSize::new(1280, 108));
        windows.borrow()[1].set_size(slint::PhysicalSize::new(360, 650));
        let shortcuts = guide::STEPS
            .iter()
            .filter(|s| !s.shortcut.is_empty())
            .map(|s| crate::DirectorShortcut {
                id: s.shortcut.into(),
                key: "F10".into(),
                label: s.title.into(),
            })
            .collect::<Vec<_>>();
        card.set_shortcuts(ModelRc::new(VecModel::from(
            shortcuts
                .iter()
                .enumerate()
                .map(|(i, s)| ShortcutRow {
                    key: s.key.clone().into(),
                    label: s.label.clone().into(),
                    guide_id: 1000 + i as i32,
                })
                .collect::<Vec<_>>(),
        )));
        let session = Rc::new(DirectorSession {
            guide_directory: Some(store.0.clone()),
            shortcuts,
            ..Default::default()
        });
        let panel = Rc::new(Cell::new(true));
        let interaction = Rc::new(Cell::new(false));
        let ready = Rc::new(Cell::new(false));
        let (window, _timer) = bind(
            session,
            &strip,
            &card,
            panel.clone(),
            interaction.clone(),
            ready.clone(),
            Rc::new(Cell::new(None)),
            0,
        )
        .unwrap()
        .unwrap();
        pump(&clock);
        assert!(!store.progress().unwrap().seen);
        ready.set(true);
        pump(&clock);
        assert!(store.progress().unwrap().seen);
        assert!(card.get_guide_active());
        assert!(interaction.get());
        card.set_keyframe_value("original".into());
        card.set_selected_keyframe_id(42);
        for index in 0..30 {
            pump(&clock);
            assert_eq!(window.get_step(), index);
            assert!(!window.get_body().is_empty());
            render(
                &windows.borrow()[2],
                &format!("guide-{:02}", index + 1),
                390,
                400,
            );
            if index == 10 || index == 23 {
                render(
                    &windows.borrow()[1],
                    &format!("card-{:02}", index + 1),
                    360,
                    650,
                );
            }
            assert_eq!(card.get_keyframe_value(), "original");
            assert_eq!(card.get_selected_keyframe_id(), 42);
            assert!(!card.get_has_selected_keyframe());
            window.invoke_action(2);
        }
        assert!(store.progress().unwrap().completed);
        assert!(!interaction.get());
        assert!(!card.get_guide_active());
        let mut prefs = store.preferences().unwrap();
        prefs.request = guide::Request::Start;
        store.save_preferences(&mut prefs).unwrap();
        pump(&clock);
        assert_eq!(window.get_step(), 0);
        window.invoke_action(2);
        window.invoke_action(3);
        assert!(!card.get_guide_active());
        prefs.request = guide::Request::Resume;
        store.save_preferences(&mut prefs).unwrap();
        pump(&clock);
        assert_eq!(window.get_step(), 1);
        assert!(card.get_guide_active());
        prefs.enabled = false;
        store.save_preferences(&mut prefs).unwrap();
        pump(&clock);
        assert!(!card.get_guide_active());
        assert!(!interaction.get());
        prefs.enabled = true;
        prefs.request = guide::Request::Reset;
        store.save_preferences(&mut prefs).unwrap();
        pump(&clock);
        assert!(!card.get_guide_active());
        // Explicit replay also works after a queued reset; compact controls remain rendered.
        card.invoke_guide_requested();
        card.set_compact_card(true);
        render(&windows.borrow()[1], "card-small", 360, 372);
        render(&windows.borrow()[2], "guide-small", 320, 360);
        let options = OptionsPreview::new().unwrap();
        options.show().unwrap();
        render(
            windows.borrow().last().unwrap(),
            "options-desktop",
            1000,
            600,
        );
        options.set_compact(true);
        render(windows.borrow().last().unwrap(), "options-small", 600, 480);
        drop(_timer);
        fs::remove_dir_all(store.0).unwrap();
    }
}
