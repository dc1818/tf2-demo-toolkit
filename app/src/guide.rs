//! Workflow metadata and persistence, independent of window geometry.
use serde::{Deserialize, Serialize};
pub const VERSION: u32 = 1;
// Step IDs stay stable so beta 38 saved progress resumes at the same topic.
pub const LAST_STEP: i32 = 39;
pub const MAX_STEP: i32 = 51;
pub const ORDER: [i32; 51] = [
    1,2,3,4,5,6,7,8,9,10,11,12,
    13,40,41,42,43,44,45,46,47,14,48,15,16,17,18,
    19,20,21,22,23,24,25,26,27,28,29,30,
    49,50,51,31,32,33,34,35,36,37,38,39,
];
pub fn ordinal(index: i32) -> i32 {
    ORDER.iter().position(|id| *id == index).unwrap_or(0) as i32 + 1
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Progress {
    pub seen_version: u32,
    pub resume_step: i32,
    pub completed: bool,
}
impl Progress {
    pub fn should_offer(&self) -> bool { self.seen_version < VERSION }
    pub fn resumable(&self) -> bool { !self.completed && (1..=MAX_STEP).contains(&self.resume_step) }
    pub fn reset_welcome(&mut self) { *self = Self::default(); }
}
#[derive(Clone, Copy, Debug)]
pub struct Step {
    pub title: &'static str,
    pub body: &'static str,
    pub page: i32,
    pub section: i32,
    pub position: i32,
    pub count: i32,
}
pub fn step(index: i32) -> &'static Step { &STEPS[index.clamp(1, MAX_STEP) as usize - 1] }
pub const STEPS: [Step; 51] = [
    Step { title: "ADD YOUR DEMOS", body: "Choose .dem files, choose a demos folder, or drag either into this box. Folders include demos in subfolders.", page: 0, section: 0, position: 1, count: 4 },
    Step { title: "CHECK YOUR SELECTION", body: "These are the demos that will be analyzed. Check their names and scroll the list if you added more than fit.", page: 0, section: 0, position: 2, count: 4 },
    Step { title: "ALREADY PARSED?", body: "Open a saved DemoToolkit_Parse_ folder to reuse its results. Choose the dated export itself, not the parent Demo Toolkit Exports folder.", page: 0, section: 0, position: 3, count: 4 },
    Step { title: "CONTINUE TO OPTIONS", body: "Your demos are ready. Choose Continue to set the export location and review the optional parse settings.", page: 0, section: 0, position: 4, count: 4 },
    Step { title: "WHERE PARSES ARE SAVED", body: "Choose where to save the analysis. The checkbox adds a Demo Toolkit Exports parent folder. Each run creates its own dated DemoToolkit_Parse_ folder.", page: 0, section: 1, position: 1, count: 4 },
    Step { title: "ITEM SCHEMA IS OPTIONAL", body: "Demo Toolkit checks for items_game.txt automatically. If it is missing, use Choose File or How to Find. You can still parse without it.", page: 0, section: 1, position: 2, count: 4 },
    Step { title: "CHOOSE PERFORMANCE", body: "Normal is the default. Low leaves more resources for other apps. High can use more CPU and memory; speed still depends on your demos and computer.", page: 0, section: 1, position: 3, count: 4 },
    Step { title: "CHECK TIME AND SPACE", body: "Review the estimated duration, export size and free space. These estimates can change. Choose Continue when the export location is valid; a schema is not required.", page: 0, section: 1, position: 4, count: 4 },
    Step { title: "REVIEW BEFORE PARSING", body: "Check the demo list and settings. Change Selection returns to your demos; Edit Options returns to your export and performance choices.", page: 0, section: 2, position: 1, count: 4 },
    Step { title: "WHAT THE APP ANALYZES", body: "It reads kills, damage, weapons, movement and round events to find highlights such as multi-kills, airshots and bookmarks. Results receive scores and searchable tags. Detail depends on the demo.", page: 0, section: 2, position: 2, count: 4 },
    Step { title: "START THE PARSE", body: "Click Parse Demos when you are ready. This creates an analysis export; it does not record video. The guide waits for parsing and analysis to finish.", page: 0, section: 2, position: 3, count: 4 },
    Step { title: "FOLLOW THE PROGRESS", body: "Watch the current stage, progress and estimated time remaining. Keep the app open. The guide continues to Candidates when results are ready.", page: 0, section: 2, position: 4, count: 4 },
    Step { title: "MEET YOUR CANDIDATES", body: "Each row is a potential clip found in a demo. We will go through every column and what its values mean, then cover searching, filtering and selecting clips.", page: 1, section: 0, position: 1, count: 15 },
    Step { title: "SEARCH MULTIPLE TAGS", body: "Search the candidate tags. Separate phrases with commas to require every phrase: airshot, medic pick. Clear the search to see all matching filters again.", page: 1, section: 0, position: 10, count: 15 },
    Step { title: "REC: RECORDING STATUS", body: "Click REC to cycle through Any, Yes and No. Any shows candidates with either recording status; Yes shows candidates with a tracked recording; No shows candidates without one. Use No to find clips still to record.", page: 1, section: 0, position: 12, count: 15 },
    Step { title: "SELECT AND INSPECT", body: "Select a row, then View Details to inspect its events, tags and score. Preview and Manual Camera need one candidate; Record Selected supports one or more.", page: 1, section: 0, position: 13, count: 15 },
    Step { title: "SECONDS BEFORE AND AFTER", body: "Before First Tick sets how many seconds are recorded before the first candidate tick. After Last Tick sets how many seconds are recorded after the last candidate tick. These values set the lead-in and ending of each clip.", page: 1, section: 0, position: 14, count: 15 },
    Step { title: "SET UP BEFORE RECORDING", body: "Before you capture anything, review Recording Settings. We will walk through all five tabs, then return here with your candidate selection preserved.", page: 1, section: 0, position: 15, count: 15 },
    Step { title: "CHECK THE PROGRAM PATHS", body: "TF2 plays the demo. HLAE provides the recording and camera tools. FFmpeg encodes supported video formats. Check detected paths and browse for any missing programs.", page: 2, section: 0, position: 1, count: 4 },
    Step { title: "CHOOSE THE VIDEO FOLDER", body: "This folder receives your captures. It is separate from the parse export folder, which stores analysis results. Choose a location with enough free space.", page: 2, section: 0, position: 2, count: 4 },
    Step { title: "FORMAT AND FRAME RATE", body: "MP4 - Standard is a simple starting point. Image sequences and editing formats suit other workflows. Higher FPS creates more frames and can increase capture time and storage.", page: 2, section: 0, position: 3, count: 4 },
    Step { title: "OPTIONAL LAUNCH OPTIONS", body: "Leave this blank unless you need extra TF2 launch options. Your settings save automatically. Continue to Video & HUD.", page: 2, section: 0, position: 4, count: 4 },
    Step { title: "SET THE CAPTURE PICTURE", body: "Resolution controls the captured image size. Skybox changes the background sky. Leave DirectX at its current value unless you know you need a different renderer.", page: 2, section: 1, position: 1, count: 3 },
    Step { title: "CHOOSE THE FIRST-PERSON VIEW", body: "Viewmodels show your weapon and arms. Viewmodel FOV changes how they appear in the frame; it is separate from the camera view used for a MIRV shot.", page: 2, section: 1, position: 2, count: 3 },
    Step { title: "CHOOSE WHAT HUD IS VISIBLE", body: "Choose how much of the in-game HUD appears in the capture. Later, Profile lets you adjust individual overlays and sounds. Continue to Encoding.", page: 2, section: 1, position: 3, count: 3 },
    Step { title: "MATCH YOUR EDITOR", body: "For MP4, start with DaVinci Resolve / Universal. Maximum Color Quality and Custom offer more control. Other formats show their own encoding options.", page: 2, section: 2, position: 1, count: 2 },
    Step { title: "ADJUST ENCODING IF NEEDED", body: "In Custom MP4, lower CRF means higher quality and larger files. Slower presets take longer. Other formats show their own codec or quality controls. Defaults are fine to start.", page: 2, section: 2, position: 2, count: 2 },
    Step { title: "SET THE GRAPHICS LOOK", body: "Maximum-quality graphics applies the recording graphics profile. Motion blur changes the appearance of movement. Choose the look you want before capturing.", page: 2, section: 3, position: 1, count: 3 },
    Step { title: "CONTROL HUD AND OVERLAYS", body: "Choose whether to show damage text, the crosshair and the 3D player model. A checked Disable option hides that element. These controls refine your HUD choice.", page: 2, section: 3, position: 2, count: 3 },
    Step { title: "CHOOSE THE CAPTURE SOUND", body: "Disable voice chat, hit sounds, announcer, applause or domination sounds if they distract from the clip. These options affect capture playback. Continue to MIRV Shortcuts.", page: 2, section: 3, position: 3, count: 3 },
    Step { title: "MOVE THROUGH THE DEMO", body: "Playback keys pause, step, seek and restart the demo while setting up a shot. The keys shown here are your current bindings. Click a binding and press a key to change it.", page: 2, section: 4, position: 4, count: 7 },
    Step { title: "BUILD A CAMERA SHOT", body: "Enter MIRV Camera to choose an angle. Add Keyframe saves a camera position at that demo time. Add more keyframes and Play Campath to move between them. Start and Stop Recording control capture.", page: 2, section: 4, position: 5, count: 7 },
    Step { title: "SAVE YOUR CAMERA WORK", body: "A campath is your saved camera path. Save it for later or load a campath XML. Director controls let you show the overlay and switch focus between Director and TF2.", page: 2, section: 4, position: 6, count: 7 },
    Step { title: "RETURN TO YOUR CANDIDATES", body: "Your recording settings are saved. Choose View Candidates to return to your selected clips. Next, we will explain Preview, Manual Camera and Record Selected.", page: 2, section: 4, position: 7, count: 7 },
    Step { title: "PREVIEW WITHOUT RECORDING", body: "With one candidate selected, Preview in TF2 opens the demo near the highlight, including your lead-in. Watch the moment before deciding what to capture.", page: 1, section: 0, position: 1, count: 5 },
    Step { title: "USE MIRV FOR CUSTOM ANGLES", body: "Manual Camera opens TF2 paused so you can choose angles and create a smooth camera path. Use it for cinematic shots beyond the original viewpoint. You decide when to start recording.", page: 1, section: 0, position: 2, count: 5 },
    Step { title: "RECORD USING YOUR SETTINGS", body: "Select one or more candidates, then Record Selected. The app records them using your Recording Settings and before/after padding. Review the recording confirmation before starting.", page: 1, section: 0, position: 3, count: 5 },
    Step { title: "TRACK WHAT YOU HAVE CAPTURED", body: "Successful captures update the Recorded status. Use REC: No to find remaining clips, and look in your capture output folder for the files.", page: 1, section: 0, position: 4, count: 5 },
    Step { title: "YOU ARE READY", body: "You can parse demos, find highlights and capture clips. Restart this guide from Options. Show Tooltips controls everyday hover help; you can turn it off whenever you like.", page: 1, section: 0, position: 5, count: 5 },
    Step { title: "RECORDED COLUMN", body: "Recorded means the app tracks a completed capture for this candidate. A blank cell means no completed recording is tracked. This helps you see which clips still need recording.", page: 1, section: 0, position: 2, count: 15 },
    Step { title: "SCORE COLUMN", body: "This is the highlight score calculated from the candidate's events and bonuses. Higher scores rank more highly. View Details shows the score breakdown so you can judge why a clip was selected.", page: 1, section: 0, position: 3, count: 15 },
    Step { title: "MAP COLUMN", body: "This is the TF2 map recorded in the source demo, such as cp_process. It tells you where this candidate happened. Use the map filter to focus on particular maps.", page: 1, section: 0, position: 4, count: 15 },
    Step { title: "CLASS COLUMN", body: "This is the class of the player whose actions form the candidate, such as Demoman or Medic. Use the class filters to find highlights for the classes you want.", page: 1, section: 0, position: 5, count: 15 },
    Step { title: "KILL TICK COLUMN", body: "These are the individual demo ticks where the candidate's kills occurred, separated by commas. Ticks are positions in the demo, not seconds. View Details lets you inspect the events behind the clip.", page: 1, section: 0, position: 6, count: 15 },
    Step { title: "SERVER TYPE COLUMN", body: "This is the server or match type inferred from the demo, such as Competitive. Unknown / Mixed means no clear type was identified. Use the server type filter to focus on the kinds of matches you want.", page: 1, section: 0, position: 7, count: 15 },
    Step { title: "TEAM COLUMN", body: "RED or BLU is the team of the player whose actions form this candidate. It is not the victim's team. This column is normally hidden in compact layouts; the guide reveals it here.", page: 1, section: 0, position: 8, count: 15 },
    Step { title: "TAGS COLUMN", body: "These grouped labels describe detected events and qualities of the highlight, such as airshots or medic picks. Use them to understand what made the moment interesting and to search for similar clips.", page: 1, section: 0, position: 9, count: 15 },
    Step { title: "FILTER YOUR CANDIDATES", body: "Use class, map and server type filters to choose which candidates are shown. Minimum Score hides candidates below that score. If nothing matches, clear the tag search or relax the filters. On small windows these controls are under Filters.", page: 1, section: 0, position: 11, count: 15 },
    Step { title: "WHY MIRV HAS HOTKEYS", body: "MIRV is HLAE's camera and recording toolkit for custom demo shots. Hotkeys let you pause, seek, move the camera, add keyframes and control capture while TF2 has focus. The bindings shown here can be customized.", page: 2, section: 4, position: 1, count: 7 },
    Step { title: "WHAT TO EXPECT FROM MIRV", body: "Manual Camera opens the selected candidate paused in TF2. Choose angles, save keyframes at demo times, then preview the camera path. Start and stop capture when ready; the clip uses your Recording Settings and output folder. Capture may run slower than real time.", page: 2, section: 4, position: 2, count: 7 },
    Step { title: "HOW THE OVERLAY WORKS", body: "Director adds a timeline across the top and a control panel on the right for seeking and editing camera keyframes. Hide/show overlay toggles the panel. Focus Director / TF2 releases the mouse for panel clicks, then returns control to TF2. These desktop windows are excluded from HLAE game capture.", page: 2, section: 4, position: 3, count: 7 },
];

pub fn has_encoding_controls(format: &str) -> bool {
    (format.starts_with("MP4") && format != "MP4 - Lossless") || format.starts_with("AVI") || format.starts_with("MOV")
}
pub fn next(index: i32, format: &str) -> i32 {
    if index == 26 && !has_encoding_controls(format) { 28 }
    else { ORDER[(ordinal(index) as usize).min(ORDER.len() - 1)] }
}
pub fn previous(index: i32, format: &str, demos: bool) -> i32 {
    if index == 28 && !has_encoding_controls(format) { 26 }
    else if index == 13 && !demos { 1 }
    else { ORDER[(ordinal(index) - 2).max(0) as usize] }
}
/// Restoring a tour cannot restore a demo/export that was not loaded this session.
pub fn resume_at(saved: i32, demos: bool, export: bool, parse_ready: bool) -> i32 {
    let saved = saved.clamp(1, MAX_STEP);
    if saved >= 13 && !export { 1 }
    else if (2..=12).contains(&saved) && !demos { 1 }
    else if (9..=12).contains(&saved) && !parse_ready { 5 }
    else if saved == 12 { 11 }
    else { saved }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn expanded_tour_navigation_is_reversible_and_keeps_saved_ids() {
        let mut visited = vec![1];
        while *visited.last().unwrap() != LAST_STEP {
            let following = next(*visited.last().unwrap(), "MP4 - Standard");
            assert!(!visited.contains(&following), "tour loops at {following}");
            visited.push(following);
        }
        assert_eq!(visited, ORDER);
        assert_eq!(next(LAST_STEP, "MP4 - Standard"), LAST_STEP);
        for pair in ORDER.windows(2) {
            assert_eq!(previous(pair[1], "MP4 - Standard", true), pair[0]);
        }
        for id in ORDER {
            assert_eq!(resume_at(id, true, true, true), if id == 12 { 11 } else { id });
        }
        assert_eq!(step(22).title, "OPTIONAL LAUNCH OPTIONS");
        assert_eq!(next(13, "MP4 - Standard"), 40);
        assert_eq!(next(30, "MP4 - Standard"), 49);
        let candidate_steps = &ORDER[12..27];
        for (position, id) in candidate_steps.iter().enumerate() {
            assert_eq!(step(*id).position, position as i32 + 1);
            assert_eq!(step(*id).count, candidate_steps.len() as i32);
            assert_eq!(step(*id).page, 1);
        }
    }
    #[test] fn onboarding_is_independent_and_versioned() {
        let mut p: Progress = serde_json::from_str("{}").unwrap();
        assert!(p.should_offer()); assert!(!p.resumable());
        p.seen_version=VERSION; p.resume_step=22;
        let loaded:Progress=serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert!(!loaded.should_offer()); assert!(loaded.resumable());
        p.completed=true; assert!(!p.resumable());
        p.reset_welcome(); assert!(p.should_offer());
    }
    #[test] fn restoring_requires_current_session_inputs() {
        assert_eq!(resume_at(22,false,false,false),1);
        assert_eq!(resume_at(22,false,true,false),22);
        assert_eq!(resume_at(9,true,false,false),5);
        assert_eq!(resume_at(12,true,false,true),11);
        assert_eq!(resume_at(999,false,true,false),MAX_STEP);
    }
    #[test] fn image_sequences_skip_hidden_encoding_controls_in_both_directions() {
        assert_eq!(next(26,"TGA Image Sequence"),28);
        assert_eq!(next(26,"MP4 - Lossless"),28);
        assert_eq!(previous(28,"JPG Image Sequence",true),26);
        assert_eq!(next(26,"MP4 - Standard"),27);
        assert_eq!(next(26,"MOV - DNxHR"),27);
        assert_eq!(previous(13,"MP4 - Standard",false),1);
    }
    #[test] fn tour_visits_all_recording_sections_before_capture() {
        let sections:Vec<_>=STEPS[18..34].iter().map(|s|s.section).collect();
        for section in 0..5 { assert!(sections.contains(&section)); }
        assert!(STEPS[18..34].iter().all(|s|s.page==2));
        assert!(STEPS[34..39].iter().all(|s|s.page==1));
        for s in STEPS { assert!(s.position>=1 && s.position<=s.count); }
    }
}
