# Windows MP4 recording validation

Tested November 14, 2026 against `84e1df760a5c749b77fddc34f00bf0c79dcb18e1`.
Windows x64 TF2, HLAE 2.189.4, FFmpeg and FFprobe build `53c21c2`.
A scratch harness used the Toolkit's batch launcher and normal recovery worker
with an isolated game copy, profile, session directory and recording index.
It recorded ticks 6000–6200 at 1280×720 as standard MP4 with H.264 and AAC.
Session data was on `C:` and the recording destination was on `E:`.
The public SourceTV fixture was demo 1530633, `cp_process_final`.
[Demo](https://demos.tf/1530633)

## Interrupted transfer recovery (V07-P3)

The first short batch published a valid MP4 and indexed it. In a replacement
batch, the harness paused the cross-volume copy after 4,096 bytes reached the
staging file, then stopped the Toolkit test process. The previous video and
the recorded index were byte-for-byte unchanged, the working capture remained
available, and the replacement's final path was absent.

On restart, the Toolkit restored the isolated TF2 profile, recovered and
indexed one clip, and removed its session with zero reported errors. The
prior file was removed only after the explicitly requested replacement was
published and indexed. FFprobe decoded the replacement. A subsequent restart
found zero sessions and left the index unchanged. The abandoned staging file
remained outside the recording index. The test covers process interruption
during a batch MP4 transfer across two NTFS volumes.

An earlier harness attempt had called recovery on the test executable's main
thread and overflowed the stack after publishing the complete video. Switching
the harness to the Toolkit's normal background recovery thread allowed it to
index that file. The full interruption-and-restart sequence above was then
repeated with the corrected harness.

## Recording FPS (V08)

FFprobe decoded the same short demo interval at both FPS settings.

| Configured FPS | Measured FPS | Decoded frames | Video duration (s) | Audio duration (s) | Audio end before video end |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 60 | 60.000 | 179 | 2.983333 | 2.949002 | 34.331 ms |
| 120 | 120.000 | 359 | 2.991667 | 2.949002 | 42.665 ms |
| 120 (repeat) | 120.000 | 358 | 2.983333 | 2.949002 | 34.331 ms |

Both streams started at timestamp zero in all three files. These measurements
establish stream timing. Sound-to-action alignment and long-clip drift still
need independent measurement. Ralph reviewed the first two measurements and
traced the setting through the batch CFG, the HLAE encoder's frame-rate
parameter, and the final FFmpeg mux, which copies the video stream. The tested
batch MP4 path honored the configured frame rate. Production code is unchanged.
Batch capture at 240/480 FPS, other presets and formats, and longer recordings
remain open for V08.

## Zero-padding single-kill capture

The three replay clips used for the V05 check were configured with both pads
at zero and one kill tick. Batch scheduling uses the kill tick over the source
clip bounds. At demo tick 20499, the generated start and stop commands both
landed at tick 20500. The recording reached both commands and produced zero
video frames. The same failure occurred in a foreground run and when seeking
was disabled for the early control.

The fix gives this specific setting two demo ticks on each side of the kill,
so the batch recorder starts one tick before it. A regression checks that the
generated VDM starts and stops on opposite sides of the kill and that ordinary
padding still gives the same bounds. All 223 Windows workspace tests passed.

| SourceTV demo 1530633 case | Decoded frames at 120 FPS | Video duration |
| --- | ---: | ---: |
| Early control, ticks 6000–6200 | 359 | 2.991667 s |
| Later focused clip, ticks 20409–20544 | 241 | 2.008333 s |
| Single kill at 20499 with zero padding after the fix | 5 | 0.041667 s |

The repaired clip was published and indexed by the normal batch flow. FFprobe
decoded its H.264 video, and a frame review showed the rocket impact. The short
clip had no measurable audio duration. File hashes and display registry values
matched the original isolated profile after the runs. [SourceTV demo](https://demos.tf/1530633)

## Manual HLAE capture at 120 and 240 FPS

Tested the manual HLAE path on Windows x64 with HLAE 2.189.4 and Toolkit
`19d31b0`. The isolated harness loaded the same SourceTV demo near tick 20409,
resumed from the manual ready point, and invoked the start and stop commands
used by the manual hotkeys. Closing TF2 triggered the normal MP4 finalizer and
profile restoration. The session reported no finalization or restore problem.
[SourceTV demo](https://demos.tf/1530633)

Both runs captured 1280×720 H.264 video with AAC audio. FFprobe counted the
frames, and FFmpeg decoded each clip end to end.

| Configured FPS | Measured FPS | Decoded video frames | Video duration | Audio duration |
| ---: | ---: | ---: | ---: | ---: |
| 120 | 120 | 233 | 1.941667 s | 1.903991 s |
| 240 | 240 | 466 | 1.941667 s | 1.903991 s |

Both streams started at zero for each run. A captured frame from the 120 FPS
run showed the expected rocket impact. The isolated TF2 configuration and
custom file hashes and display registry values matched their original snapshots
after each exit.

The harness exercised the manual capture commands and native finalization.
Manual 480 FPS, desktop hotkey input, and independent sound-to-action alignment
remain open.
