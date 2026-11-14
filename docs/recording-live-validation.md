# Windows batch MP4 validation

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
Manual capture, 240/480 FPS, other presets and formats, and longer recordings
remain open for V08.
