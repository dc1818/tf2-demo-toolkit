# TF2 Demo Director

TF2 Demo Director is the camera companion for TF2 Demo Toolkit's manual HLAE recording mode. Select a candidate and launch a manual session from the main app; it opens the Director with that clip's timeline and camera controls.

The timeline sits across the top of the screen, and the control panel sits on the right. These are desktop windows, so HLAE's game capture does not include them. The saved panel shortcut (`C` by default) hides or restores the control panel.

Use the Director to seek through a clip, add and edit campath keyframes, preview the path, and save or load campath XML files. The main application handles the recording profile, output files, and restoring TF2 settings when the session ends.

## Command delivery

The Director writes actions to temporary CFG files. TF2 reads them through a guarded `wait` loop and acknowledges each action in its console log. Camera state is read back with `mirv_campath print`. If TF2 disables `wait`, the Windows fallback presses the dedicated command bind while TF2 retains focus.

The companion does not inject another DLL. HLAE still supplies the `mirv_*` commands used by the recording session.

## Build and run

Both repository build scripts package the Director alongside the main application. See the [main README](../README.md#build-from-source) for prerequisites and build instructions.

For troubleshooting, a built Director can also open an existing session file directly:

```sh
tf2-demo-director path/to/director_session.json
```

The packaged executable is `TF2_Demo_Director.exe` on Windows and `TF2_Demo_Director` on Linux and macOS. The desktop app builds on all three platforms; TF2/HLAE recording integration is Windows-only.
