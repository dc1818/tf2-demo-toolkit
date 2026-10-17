//! Independent Director onboarding. Toolkit owns preferences; Director owns progress.
use crate::DirectorShortcut;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
pub enum Request {
    #[default]
    None,
    Start,
    Resume,
    Reset,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Preferences {
    pub enabled: bool,
    pub request: Request,
    pub revision: u64,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            enabled: true,
            request: Request::None,
            revision: 0,
        }
    }
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Progress {
    pub seen: bool,
    pub completed: bool,
    pub step: usize,
    pub handled_revision: u64,
}
#[derive(Clone)]
pub struct Store(pub PathBuf);
impl Store {
    pub fn preferences(&self) -> Result<Preferences> {
        read(&self.0.join("preferences.json"))
    }
    pub fn progress(&self) -> Result<Progress> {
        read(&self.0.join("progress.json"))
    }
    pub fn save_progress(&self, value: &Progress) -> Result<()> {
        write(&self.0.join("progress.json"), value)
    }
    pub fn save_preferences(&self, value: &mut Preferences) -> Result<()> {
        value.revision = (SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64)
            .max(value.revision.saturating_add(1));
        write(&self.0.join("preferences.json"), value)
    }
}
fn read<T: serde::de::DeserializeOwned + Default>(path: &Path) -> Result<T> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .with_context(|| format!("Invalid guide state: {}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(e) => Err(e.into()),
    }
}
fn write(path: &Path, value: &impl Serialize) -> Result<()> {
    fs::create_dir_all(path.parent().context("Guide directory missing")?)?;
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&temp, serde_json::to_vec_pretty(value)?)?;
    replace(&temp, path).with_context(|| format!("Cannot save guide state: {}", path.display()))
}
#[cfg(not(windows))]
fn replace(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::rename(from, to)
}
#[cfg(windows)]
fn replace(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
    }
    let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 1 | 8) } == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
/// Reset is deferred until the next process. Ordinary first use starts only after UI readiness.
pub fn requested_step(p: &Preferences, s: &Progress, startup: bool) -> Option<usize> {
    if !p.enabled {
        return None;
    }
    if p.revision != s.handled_revision {
        match p.request {
            Request::Start => return Some(0),
            Request::Resume => {
                return Some(if s.completed {
                    0
                } else {
                    s.step.min(STEPS.len() - 1)
                })
            }
            Request::Reset => return startup.then_some(0),
            Request::None => (),
        }
    }
    (!s.seen).then_some(0)
}
pub struct Step {
    pub title: &'static str,
    pub body: &'static str,
    pub shortcut: &'static str,
}
pub const STEPS: &[Step] = &[
    Step { title: "Welcome to MIRV Director", body: "Build a camera path with keyframes, preview it, then record. The timeline follows the demo and the side panel holds frag cues and controls. Recording Settings are already active. Hotkeys let you control playback, the camera and recording while TF2 has focus. Director is a separate desktop overlay; whether it appears in output depends on your capture method.", shortcut: "" },
    Step { title: "Click the overlay", body: "Release TF2 input to click Director controls, then use the same shortcut to return to the game. This guide starts in click mode and pauses when you return to TF2.", shortcut: "overlay_interaction_toggle" },
    Step { title: "Read the timeline", body: "CLIP and OUT mark the recording range. Blue shows playback progress, green marks camera keyframes, and orange marks frag cues. Click a marker to inspect it.", shortcut: "" },
    Step { title: "Follow live ticks", body: "CURRENT TICK is the demo position reported by TF2. LIVE means telemetry is arriving; LOADING, NO TICK or NO LOG means the position is not currently updating. TOPMOST keeps the overlay above the game.", shortcut: "" },
    Step { title: "Jump to the next frag", body: "The card shows the next frag tick, victim and tags. GO TO FRAG (-1 SEC) seeks to one second before that frag so you can set up the shot.", shortcut: "" },
    Step { title: "Pause or resume", body: "Pause or resume demo playback. Pause when positioning a camera or inspecting a keyframe.", shortcut: "pause_resume" },
    Step { title: "Advance a little", body: "Advance the paused demo by 0.25 seconds to choose a precise moment for a camera keyframe.", shortcut: "advance_time" },
    Step { title: "Step back", body: "Seek back one second to revisit the action. Wait for the live tick to settle before continuing.", shortcut: "back_one_second" },
    Step { title: "Restart the clip", body: "Reload the demo and seek back to the clip start. Expect a loading wait; save your camera path before restarting.", shortcut: "safe_restart" },
    Step { title: "Move between frags", body: "Seek to one second before the next frag cue. Use this to plan shots around each kill in the candidate.", shortcut: "next_kill_tick" },
    Step { title: "Position your camera", body: "Enter the MIRV camera and move to the angle you want. The camera uses the configured starting FOV. Arrow keys move the camera.", shortcut: "enter_camera" },
    Step { title: "Add a keyframe", body: "Store the current camera position, angles, FOV and time. Move through the demo and add more keyframes to build a camera path.", shortcut: "add_keyframe" },
    Step { title: "Preview your camera path", body: "Enable campath playback, then resume the demo to watch the camera move between your keyframes. Preview the path before recording.", shortcut: "play_campath" },
    Step { title: "Show the path", body: "Show or hide the campath and keyframe IDs in the game. Hide these visual aids before capture.", shortcut: "draw_campath" },
    Step { title: "Refresh the markers", body: "Ask HLAE for the current keyframes again. Use this if the timeline needs to catch up after a path change.", shortcut: "print_keyframes" },
    Step { title: "Select a keyframe", body: "Click a green marker or a keyframe row to open its editor. The number is the HLAE keyframe ID; the tick tells you where it occurs in the demo.", shortcut: "" },
    Step { title: "Choose an action", body: "Edit a keyframe, go to its tick or one second before it, delete it, or change the selected set. Select range and Add range use the ending ID you enter. Select all, Deselect all and Invert selection change the selected set.", shortcut: "" },
    Step { title: "Edit, then execute", body: "Choose a setting, then enter a value if needed. EXECUTE HLAE ACTION applies the selected action. Position, angles, time and FOV can be changed independently. Clicking away closes the editor.", shortcut: "" },
    Step { title: "Change time and framing", body: "Time moves a keyframe to the current moment, shifts it, or sets an absolute time. Position, angles and FOV can use the current camera or explicit values. An asterisk keeps an axis unchanged.", shortcut: "" },
    Step { title: "Adjust path movement", body: "Rotate changes orientation. Anchor and path offset align the path using the current camera or timing. Duration changes the timing span. Review the path after each adjustment.", shortcut: "" },
    Step { title: "Choose interpolation", body: "Position, rotation and FOV have separate interpolation choices. Default uses HLAE defaults; linear makes straight transitions and cubic smooths the curve. Execute the action, then preview the result.", shortcut: "" },
    Step { title: "Save your camera path", body: "Save the campath as an XML file before leaving this session. You can reuse or edit it later. Saving the path does not record a video.", shortcut: "save_campath" },
    Step { title: "Load an existing path", body: "Open the file picker and choose a saved camera path. Review its keyframes and timing against this demo before recording.", shortcut: "load_campath" },
    Step { title: "Start recording", body: "Start capture using the format, FPS, graphics and output settings selected in Toolkit. Enable the campath and control demo playback for the shot.", shortcut: "start_recording" },
    Step { title: "Stop recording", body: "Stop the current capture. Let Toolkit finish preparing or transferring the output before opening it. Stopping capture does not close TF2.", shortcut: "stop_recording" },
    Step { title: "Toggle the HUD", body: "Show or hide the in-game HUD for the shot. This changes what TF2 renders; the Director panel is a separate desktop overlay.", shortcut: "toggle_hud" },
    Step { title: "Show controls", body: "Display the manual-session control reference in TF2. The keys shown by this guide and Director must reflect your configured shortcuts.", shortcut: "show_help" },
    Step { title: "Hide or show the overlay", body: "Toggle the Director panel, timeline and this guide together. Show the overlay again to continue at this step. Hiding the overlay does not stop capture or close Director.", shortcut: "overlay_panel_toggle" },
    Step { title: "Command feedback and fallback", body: "Watch the command status for delivery feedback. Actions normally execute automatically. If TF2 reports that wait is unavailable, the configured emergency fallback key executes the queued action while TF2 has focus.", shortcut: "execute_director_action" },
    Step { title: "Finish and restore", body: "CLOSE TF2 ends the game session. Toolkit finishes the output and restores your original config. Check the Toolkit status area for Original config restored.", shortcut: "" },
];
pub fn body(step: usize, shortcuts: &[DirectorShortcut]) -> String {
    let step = &STEPS[step.min(STEPS.len() - 1)];
    let key = shortcuts.iter().find(|key| key.id == step.shortcut);
    match key {
        Some(key) => format!("{}\n\nYour shortcut: {}", step.body, key.key),
        None => step.body.into(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_use_disable_resume_and_reset() {
        let mut p = Preferences::default();
        let mut s = Progress::default();
        assert_eq!(requested_step(&p, &s, true), Some(0));
        p.enabled = false;
        assert_eq!(requested_step(&p, &s, true), None);
        p.enabled = true;
        s.seen = true;
        s.step = 12;
        assert_eq!(requested_step(&p, &s, true), None);
        p.request = Request::Resume;
        p.revision = 1;
        assert_eq!(requested_step(&p, &s, false), Some(12));
        s.handled_revision = 1;
        assert_eq!(requested_step(&p, &s, false), None);
        p.request = Request::Reset;
        p.revision = 2;
        assert_eq!(requested_step(&p, &s, false), None);
        assert_eq!(requested_step(&p, &s, true), Some(0));
        p.request = Request::Start;
        assert_eq!(requested_step(&p, &s, false), Some(0));
    }
    #[test]
    fn state_writers_are_independent_and_corruption_is_reported() {
        let store =
            Store(std::env::temp_dir().join(format!("director-guide-test-{}", std::process::id())));
        let _ = fs::remove_dir_all(&store.0);
        let mut p = Preferences::default();
        p.enabled = false;
        store.save_preferences(&mut p).unwrap();
        store
            .save_progress(&Progress {
                seen: true,
                step: 17,
                ..Default::default()
            })
            .unwrap();
        assert!(!store.preferences().unwrap().enabled);
        assert_eq!(store.progress().unwrap().step, 17);
        p.request = Request::Reset;
        store.save_preferences(&mut p).unwrap();
        assert_eq!(store.progress().unwrap().step, 17);
        fs::write(store.0.join("progress.json"), b"broken").unwrap();
        assert!(store.progress().is_err());
        fs::remove_dir_all(store.0).unwrap();
    }
    #[test]
    fn all_steps_and_configured_hotkeys() {
        assert_eq!(STEPS.len(), 30);
        let key = DirectorShortcut {
            id: "overlay_interaction_toggle".into(),
            key: "F10".into(),
            label: "Clicks".into(),
        };
        assert!(body(1, &[key]).contains("F10"));
        assert!(!body(1, &[]).contains("F11"));
        assert!(STEPS
            .iter()
            .all(|s| !s.body.is_empty() && !s.title.is_empty()));
    }
}
