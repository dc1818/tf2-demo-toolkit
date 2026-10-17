# MIRV Director guide implementation

## Plan

1. Preserve the existing recording-profile backup/restoration paths and expose their verified state in the Toolkit header, alongside TF2 and Director activity.
2. Add a separate, resumable Director tutorial that begins only after its windows and the demo are ready. Use the existing TF2 typography, dark panels, gold highlights and muted red navigation.
3. Cover the overlay, configured hotkeys, timeline, camera path, keyframe editor, recording, feedback and shutdown in 30 steps. Guide navigation must never dispatch game commands.
4. Add independent Enable, Start, Resume and Reset controls to Options. Persist Director onboarding independently of the Toolkit Quick Guide.
5. Compile both applications, test first-use and preference transitions, render production UI at desktop and compact sizes, then publish and verify the beta packages.

## Implementation

The session includes an optional `guide_directory`; older session files remain valid. Toolkit writes `Director Guide/preferences.json`, while Director writes `progress.json`, preventing cross-process overwrites of application settings. JSON replacement is atomic on supported desktop platforms. Corrupt state is reported rather than silently reset.

A separate guide window highlights the relevant live controls. It uses configured shortcut labels and saves the current step. Back and Next change only the tour. Pause/Escape retains progress; Skip or Finish dismisses it. Enable defaults to true for first use. Start and Resume can be queued before Director is ready. Reset applies on the next Director process, not halfway through the current session.

Entering the tour temporarily enables overlay clicks and shows the panel. Leaving restores the previous state. Returning input to TF2 pauses the guide. Hiding the overlay hides the panel, timeline and active guide together; showing it again restores the same guide step and click mode. Demo reload pauses the tour. Conditional editor explanations show a labeled, noninteractive example when no keyframe is selected; the tour never adds or changes real keyframes.

The Toolkit header is driven by the existing profile staging and verified restoration code, TF2 process state, and a Director UI heartbeat. It does not claim restoration while a recovery pointer is still pending.

## Validation

Automated tests cover independent state writers, invalid state, first use, disabling, explicit start/resume/reset, configured hotkeys, completion, preservation of keyframe values, and rendered production guide/Options screens. The complete app is compiled as well as the Director library.

Native Windows focus transfer, mixed-DPI monitor movement and live TF2 recording need verification on a Windows gaming setup. Headless UI tests do not establish those runtime behaviors.

## Overlay flashing and HUD prompt fix

The previous implementation raised the panel/timeline and guide from independent 250 ms timers and requested the same guide size/position on every tick. The common topmost helper also used `SWP_SHOWWINDOW`, which could resurrect a hidden timeline. These are code-level causes consistent with the reported flashing; no live TF2 trace was available.

One controller now repairs Windows stacking only when necessary, batching the windows in timeline/panel/guide order without activation or show flags. Guide placement requests only changed native geometry. The global visibility action hides/restores all windows together, retains the active guide step, and keeps the event loop running while everything is hidden.

[Valve's HUD implementation](https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/game/client/tf/tf_hud_playerstatus.cpp) gates the 3D HUD prompt on `cl_hud_playerclass_playermodel_showed_confirm_dialog`. Temporary startup/profile/capture settings set it to 1 before the model preference is applied. The original model choice and the existing verified config backup/restoration mechanism are preserved.
