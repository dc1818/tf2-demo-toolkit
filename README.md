# TF2 Demo Toolkit

A desktop tool for finding and recording clips from Team Fortress 2 demos. It parses `.dem` files, ranks possible highlights, and lets you preview or record the clips you want to keep.

I built this for working through demo collections without having to scrub through every match by hand. For shots that need a different angle, the manual MIRV workflow includes a Director overlay for placing and editing camera keyframes.

The application is written in Rust with a Slint interface. Parsing and candidate browsing work on Windows, Linux, and macOS. HLAE recording and manual MIRV sessions are Windows-only.

## Features

- Batch parsing with progress, time estimates, and a disk-space check before starting.
- Ranked candidates with kill ticks, player and map information, and tags for individual kills.
- Filters for class, map, server type, tags, score, and recorded status.
- Support for POV and STV demos, including bookmarks from `ds_mark`.
- Preview a selected clip in TF2 or record a batch with HLAE.
- MP4, MOV/DNxHR, AVI, TGA, and JPG output, with 60/120/240/480 FPS options.
- Manual camera paths with a keyframe timeline, editing controls, and XML save/load.
- Recording tracking to help avoid duplicate clips.
- Temporary recording profiles, TF2 configuration backups, and interrupted-session recovery.

Scores and tags are a way to narrow down the footage, not a substitute for watching it. Airshot detection and server-type classification use demo evidence and can still get things wrong.

## Getting started

### Windows package

Open [Actions → Rust workspace](https://github.com/dc1818/tf2fragdemohelper/actions/workflows/build.yml), select a successful run for `main`, and download the **TF2-Demo-Toolkit-Windows** artifact. GitHub may require you to sign in to download it.

Extract the entire ZIP and keep these items together:

- `TF2_Demo_Toolkit.exe` — the main application.
- `TF2_Demo_Director.exe` — the manual camera overlay.
- `export_all.exe` — the demo parser.
- `recording_resources_archive/` — bundled recording resources.

Run `TF2_Demo_Toolkit.exe`. Python and .NET are not required.

### Recording requirements

To preview or record demos, you need an installed copy of TF2 and the demo's map. HLAE recording also needs:

- [HLAE](https://www.advancedfx.org/download/) extracted with its hook files intact.
- [FFmpeg](https://ffmpeg.org/download.html) for encoded video output.
- Enough free space for the working captures as well as the finished clips.

Set the TF2 executable, HLAE executable, FFmpeg executable, and recording output folder in **Recording Settings**. For current 64-bit TF2 installations, select `tf_win64.exe`.

**Recording sessions are for offline demo playback only.** The helper launches TF2 with `-insecure` and `+sv_lan 1`. Do not join matchmaking or community servers from that instance. HLAE uses its normal game hook; the Director does not add another injected DLL.

## Find clips

1. On **Parse Demos**, choose your demos or a folder, or drag them into the window.
2. Choose an export folder and performance profile. Review the time and space estimate before starting.
3. Open **Candidates** when analysis finishes.
4. Filter the list and use **View Details** to inspect a candidate's kills, tags, and score breakdown.
5. Select one candidate and use **Preview Selected in TF2** to check the footage.

You can reopen an export with **Load Previously Parsed Export** instead of parsing the same demos again. Keep the original `.dem` files available: exports contain analysis, not a replacement for the footage.

POV demos only contain what was available to the recording player. STV demos can provide candidates from multiple players. Neither format guarantees that every player or event has enough data for every analysis rule.

## Record clips

Choose the format, resolution, FPS, and lead-in/outro in **Recording Settings**, then select candidates and use **Record with HLAE**.

The default is **MP4 - Standard**, using H.264 and AAC. Advanced encoding options include lossless AVI codecs, MP4 quality and compatibility controls, and DNxHR profiles. JPG quality is available when JPG output is selected.

Automatic recording uses the original POV camera, or the candidate's attacker in-eye view for STV demos. For a custom camera angle, use a manual MIRV session instead.

The helper checks recording space before starting, tracks completed outputs, and asks how to handle candidates that have already been recorded. If you choose to re-record, the old output is kept until its replacement finishes successfully.

### Output folders

Within the selected recording folder:

| Capture | Location |
| --- | --- |
| Automatic video | `Videos/<Primary Tag>/` |
| Automatic TGA/JPG sequence | `Image Sequences/<clip>/Frames/`, with WAV audio under `Audio/` |
| Manual MIRV capture | `Videos/Manual HLAE/<Primary Tag>/` |

Temporary captures and recovery files are kept separately from finished outputs. Successful batch sessions are cleaned up after finalization and TF2 restoration; interrupted sessions are retained for recovery.

## Manual MIRV and Director

Select exactly one candidate and click **Launch Manual MIRV Session**. TF2 loads the demo from tick 0, seeks forward in stages, and pauses at your selected lead-in. The HUD starts off.

Director opens a timeline above TF2 and a panel on the right. Use them to navigate frags, inspect keyframes, edit a selected keyframe, load a saved campath, or close TF2.

Default shortcuts:

| Key | Action |
| --- | --- |
| `[` | Advance about 0.25 seconds |
| `]` | Toggle HUD |
| `1` | Show shortcut help in the console |
| `2` | Go back about one second |
| `3` | Restart safely and return to the lead-in |
| `4` | Next frag, one second early; wraps to the first frag |
| `5` | Pause/resume |
| `6` | Enter MIRV camera and reset live FOV to the recording setting |
| `7` | Add keyframe |
| `8` | End manual input and enable campath playback |
| `9` / `0` | Start / stop recording |
| `-` | Print keyframes |
| `=` | Save campath XML |
| `F8` | Choose and load campath XML |
| `C` | Hide/show Director panel |
| `F11` | Switch focus between Director and TF2 |

Shortcuts can be changed in settings. Arrow keys are left available for MIRV camera movement.

A basic workflow is:

1. Press `6`, position the camera, and press `7`.
2. Advance the demo and repeat for the important moments.
3. Press `8`, then `3` to return to the lead-in, and `5` to preview.
4. Save the path with `=` when you are happy with it.
5. For recording, return to the lead-in with `3`, enable the path with `8`, then use **`5` → `9` → `0`**: resume, start recording, stop recording.
6. Close TF2 and let the helper finish processing the capture.

Editing a keyframe's FOV targets that keyframe's HLAE ID, not the whole path. Pressing `6` resets the live camera FOV without changing existing keyframes.

Same-session safe restarts keep the campath in memory. Saving creates an XML file; it does not automatically load that path in a future session. Use `F8` or the panel's load button to bring it back.

For detailed camera controls and POV troubleshooting, see the [manual HLAE camera guide](MANUAL_HLAE_CAMERA_GUIDE.md). Director's command delivery is described in [director/README.md](director/README.md).

### Manual launch options

**Manual Launch Options** accepts extra Source launch parameters such as `-high -nojoy` or `+mat_queue_mode 2`. The format checker rejects malformed input and options that conflict with the helper's offline settings, demo loading, or session controls.

A valid format does not mean an option is useful or supported by TF2. Leave this field empty unless you need a particular option.

## TF2 settings and recovery

Recording temporarily changes TF2's configuration and selected custom resources. The helper backs up the original CFG folder, binds, HUD/custom content, hitsounds, video settings, and DX setting, then restores them after its TF2 session closes.

Keep the helper open while recording and finalizing. If TF2 crashes or a batch is interrupted, the helper tries to finalize usable captures and preserve unfinished work. Reopening the helper runs recovery for retained sessions.

Do not manually delete session backups while restoration or recovery is pending. Check the **Logs** page if recording, finalization, or restoration fails.

## Build from source

Install [Rust](https://rustup.rs/). The workspace requires Rust 1.88 or newer; GitHub Actions uses 1.88.0.

On Windows, the MSVC toolchain also needs Visual Studio Build Tools or Visual Studio Community with **Desktop development with C++** and a Windows SDK installed. VS Code alone does not provide `link.exe`.

```sh
git clone https://github.com/dc1818/tf2fragdemohelper.git tf2-demo-toolkit
cd tf2-demo-toolkit
```

### Windows

Run `BUILD_RUST_APP.bat`. The script builds the workspace and packages TF2 Demo Toolkit, TF2 Demo Director, the parser, and recording resources in `dist/`.

### Linux and macOS

Run:

```sh
sh build_rust_app.sh
```

On Debian/Ubuntu, install the GUI dependency first:

```sh
sudo apt-get install libfontconfig1-dev
```

The desktop application builds on these platforms, but TF2/HLAE recording integration is Windows-only.

Existing installations keep their settings, recording history, and recovery data. The internal settings and backup folders retain their original names for compatibility.

### Checks

```sh
cargo check --workspace --all-targets
cargo test --workspace
```

GitHub Actions runs workspace checks and tests on Windows, Ubuntu, and macOS, and builds the Windows release package.

## Source layout

| Directory | Contents |
| --- | --- |
| `app/` | Desktop UI, analysis, filtering, recording, settings, and recovery |
| `parser/` | TF2 demo parser library and `export_all` binary |
| `director/` | MIRV Director companion and overlay UI |
| `app/ui/` | Slint components, theme, fonts, and class icons |
| `recording_resources_archive/` | Bundled recording resources |
| `.github/workflows/` | Cross-platform checks and Windows packaging |

The interface uses a TF2-inspired theme with full and compact layouts. The [Figma design](https://www.figma.com/design/Yr10mYuw4jcnQCMKuTPCH3) is available as a visual reference.

## Reporting issues

Use [GitHub Issues](https://github.com/dc1818/tf2fragdemohelper/issues). Include the build or commit you used, the steps to reproduce the problem, the demo type (POV/STV), and any relevant logs. For recording problems, include your format, FPS, and HLAE version.

When possible, include a small demo that reproduces the issue. Remove private paths, player information, or anything else you do not want to share before uploading logs or demos.

## Credits and third-party assets

The parser is based on [demostf/parser](https://codeberg.org/demostf/parser) and is licensed under MIT or Apache-2.0. Recording assets come from the [Lawena Recording Tool](https://github.com/quanticc/lawena-recording-tool), with additional fonts and Valve class artwork used in the interface.

See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for asset credits and licensing details. HLAE and FFmpeg are separate downloads and are not bundled. This is a fan-made utility, not an official Valve tool.
