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

Entering the tour temporarily enables overlay clicks and shows the panel. Leaving restores the previous state. Returning input to TF2 or hiding the panel pauses the guide and respects that user choice. Demo reload pauses the tour. Conditional editor explanations show a labeled, noninteractive example when no keyframe is selected; the tour never adds or changes real keyframes.

The Toolkit header is driven by the existing profile staging and verified restoration code, TF2 process state, and a Director UI heartbeat. It does not claim restoration while a recovery pointer is still pending.

## Validation

Automated tests cover independent state writers, invalid state, first use, disabling, explicit start/resume/reset, configured hotkeys, completion, preservation of keyframe values, and rendered production guide/Options screens. The complete app is compiled as well as the Director library.

Native Windows focus transfer, mixed-DPI monitor movement and live TF2 recording need verification on a Windows gaming setup. Headless UI tests do not establish those runtime behaviors.
