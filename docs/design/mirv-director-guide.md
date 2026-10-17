# MIRV Director guide

Status: implemented. See `mirv-director-implementation.md` for behavior and validation. Shortcut names in this design table describe defaults; the running guide uses the configured keys.

## First use and navigation

- Start once the Director windows and demo-ready state are available, on the user's first Director use. Never mark the tour seen just because a companion process was launched.
- Enter click mode for the tour so its buttons are operable; preserve and restore the previous interaction/panel state when leaving the tour.
- Persist first-use/completed/paused state separately from the main Toolkit guide. Offer a replay entry in Director and Options.
- Back, Skip and Next navigate explanations only. Finish closes the tour. They do not launch TF2, issue HLAE commands, begin capture, delete keyframes or close the game.
- Use actual configured shortcut labels, never hard-coded default keys in the implemented guide. The mockups show defaults.
- One focused target per step; scroll the shortcut list to the target. Keep the target and tooltip on-screen. Use the same gold outline, pointer, fonts, colors and buttons as the existing Toolkit guide.
- Conditional keyframe-editor steps need a clear demo illustration if no real keyframe exists; do not create or edit the user's campath to make the tour work.
- Keep overlays and guide windows separate from the TF2 capture surface. Verify the selected capture backend before promising that no desktop overlay can appear in output.

## Step copy

| Step | Title | Target | Copy |
| --- | --- | --- | --- |
| 1 | Welcome to MIRV Director | Whole overlay | Build a camera path with keyframes, preview it, then record. The timeline follows the demo and the side panel holds frag cues and controls. Recording Settings are already active. |
| 2 | Click the overlay | Interaction toggle | F11 releases TF2 input so you can click Director controls. Press it again to return to the game. The tour begins in click mode so its buttons work. |
| 3 | Read the timeline | Timeline | CLIP and OUT mark the recording range. Blue shows playback progress, green marks camera keyframes, and orange marks frag cues. Click a marker to inspect it. |
| 4 | Follow live ticks | LIVE / current tick | CURRENT TICK is the demo position reported by TF2. LIVE means telemetry is arriving; LOADING, NO TICK or NO LOG means the position is not currently updating. TOPMOST keeps the overlay above the game. |
| 5 | Jump to the next frag | Frag cue and Go to frag | The card shows the next frag tick, victim and tags. GO TO FRAG (-1 SEC) seeks to one second before that frag so you can set up the shot. |
| 6 | Pause or resume | pause_resume | Pause or resume demo playback. Pause when positioning a camera or inspecting a keyframe. |
| 7 | Advance a little | advance_time | Advance the paused demo by 0.25 seconds to choose a precise moment for a camera keyframe. |
| 8 | Step back | back_one_second | Seek back one second to revisit the action. Wait for the live tick to settle before continuing. |
| 9 | Restart the clip | safe_restart | Reload the demo and seek back to the clip start. Expect a loading wait; save your camera path before restarting. |
| 10 | Move between frags | next_kill_tick | Seek to one second before the next frag cue. Use this to plan shots around each kill in the candidate. |
| 11 | Position your camera | enter_camera | Enter the MIRV camera and move to the angle you want. The camera uses the configured starting FOV. Arrow keys move the camera. |
| 12 | Add a keyframe | add_keyframe | Store the current camera position, angles, FOV and time. Move through the demo and add more keyframes to build a camera path. |
| 13 | Preview your camera path | play_campath | Enable campath playback, then resume the demo to watch the camera move between your keyframes. Preview the path before recording. |
| 14 | Show the path | draw_campath | Show or hide the campath and keyframe IDs in the game. Hide these visual aids before capture. |
| 15 | Refresh the markers | print_keyframes | Ask HLAE for the current keyframes again. Use this if the timeline needs to catch up after a path change. |
| 16 | Select a keyframe | Timeline marker / keyframe row | Click a green marker or a keyframe row to open its editor. The number is the HLAE keyframe ID; the tick tells you where it occurs in the demo. |
| 17 | Choose an action | Keyframe action menu | Edit a keyframe, go to its tick or one second before it, delete it, or change the selected set. Select range and Add range use the ending ID you enter. Select all, Deselect all and Invert selection change the selected set. |
| 18 | Edit, then execute | Setting / value / Execute | Choose a setting, then enter a value if needed. EXECUTE HLAE ACTION applies the selected action. Position, angles, time and FOV can be changed independently. Clicking away closes the editor. |
| 19 | Change time and framing | Time / position / angles / FOV options | Time moves a keyframe to the current moment, shifts it, or sets an absolute time. Position, angles and FOV can use the current camera or explicit values. An asterisk keeps an axis unchanged. |
| 20 | Adjust path movement | Rotate / anchor / path offset / duration | Rotate changes orientation. Anchor and path offset align the path using the current camera or timing. Duration changes the timing span. Review the path after each adjustment. |
| 21 | Choose interpolation | Interpolation options | Position, rotation and FOV have separate interpolation choices. Default uses HLAE defaults; linear makes straight transitions and cubic smooths the curve. Execute the action, then preview the result. |
| 22 | Save your camera path | save_campath | Save the campath as an XML file before leaving this session. You can reuse or edit it later. Saving the path does not record a video. |
| 23 | Load an existing path | Load campath XML | Open the file picker and choose a saved camera path. Review its keyframes and timing against this demo before recording. |
| 24 | Start recording | start_recording | Start capture using the format, FPS, graphics and output settings selected in Toolkit. Enable the campath and control demo playback for the shot. |
| 25 | Stop recording | stop_recording | Stop the current capture. Let Toolkit finish preparing or transferring the output before opening it. Stopping capture does not close TF2. |
| 26 | Toggle the HUD | toggle_hud | Show or hide the in-game HUD for the shot. This changes what TF2 renders; the Director panel is a separate desktop overlay. |
| 27 | Show controls | show_help | Display the manual-session control reference in TF2. The keys shown by this guide and Director must reflect your configured shortcuts. |
| 28 | Hide or show the panel | overlay_panel_toggle | Toggle the side panel while keeping the top timeline visible. Hiding the panel does not stop capture or close Director. |
| 29 | Command feedback and fallback | Command status / execute_director_action | Watch the command status for delivery feedback. Actions normally execute automatically. If TF2 reports that wait is unavailable, the configured emergency fallback key executes the queued action while TF2 has focus. |
| 30 | Finish and restore | Close TF2 | CLOSE TF2 ends the game session. Toolkit finishes the output and restores your original config. Check the Toolkit status area for Original config restored. |

## Mockup sheets

The twelve representative scenes show steps 1, 2, 3, 5; 11, 13, 16, 18; and 22, 23, 24, 30. Generated scenes illustrate placement and styling; their game backdrops are not captured TF2 screenshots. The complete step list above covers the controls omitted from those representative scenes.
