# V07 image-sequence completion proposal v1

**Approved in chat on November 26, 2026.** The owner also approved keeping
unindexed legacy image sessions pending when completion evidence is absent.
This proposal covers automatic JPG and TGA batches.
Ralph reviewed the capture, restart recovery, and replacement paths at pilot
checkpoint `6553f2f`. The proposed validation uses the existing disposable
Windows fixtures.

## Evidence

| Clip | Native console evidence | Media on disk | Result at the pilot baseline |
| --- | --- | --- | --- |
| Complete TGA | START, END, engine stop acknowledgement, then FINALIZED for that clip | 180 consecutive frames and a decodable 2.995-second WAV | Recorded after restart |
| Interrupted TGA | START only | 13 consecutive frames and a WAV that failed strict decoding | Incorrectly Recorded after restart |
| Unstarted TGA | No clip markers | No capture artifacts | RecoveryPending |
| Normal JPG control | Toolkit reported one completed candidate | 180 decodable frames, with no WAV emitted in this run | Recorded |

The incomplete TGA sequence contained structurally valid frames. Its WAV held
36,864 bytes after the header, but the WAV data chunk declared zero bytes.
The complete clip's WAV declared the same data length as the file provided.
The production stop command echoes END before calling `endmovie`, so the
engine acknowledgement and later marker matter. Live early-exit consolidation
and restart recovery currently accept an image artifact as a basis for
finalization. Restart also has an expected-output fallback that can accept a
partially populated destination. [Live fixture results](recording-recovery-validation.md#live-windows-image-capture-and-restart-recovery)

## Proposed behavior

1. **Record completion for the exact clip occurrence.** Use a session-owned
   console log for automatic image batches. Give each clip occurrence markers
   keyed by its unique capture prefix, including repeated candidates. Require
   ordered START, END, the engine's movie-stop acknowledgement for that active
   clip, and its later FINALIZED marker. Parse complete lines and exact tokens.
   Earlier live logs sometimes interleave the words of multi-token markers.
   Retain the session evidence across restart. Save an atomic completion
   receipt when the monitor observes the sequence. Startup recovery can
   rebuild the receipt from the same session-owned log if the Toolkit stopped
   before its monitor wrote the receipt.
2. **Require that proof before publishing.** Use the same check for normal
   image finalization, early TF2 exit, and restart recovery, including the
   expected-output fallback. Check for a contiguous, nonempty sequence in
   the selected frame format. When WAV is present, require structurally valid,
   nonempty audio data that fits within the file. JPG sequences may have no
   WAV. These checks use the native file structures. The test harness decodes
   the complete output independently.
3. **Preserve salvage and replacements.** Leave unproven captures pending and
   keep their original frame and audio bytes available with the session
   diagnostics. Keep a confirmed sequence outside the discoverable output
   folders while transferring its files. Publish the complete directory to
   its existing final path, then write the index. A failed or interrupted
   transfer remains retryable without exposing an incomplete sequence to
   export reconciliation. Remove a superseded recording only after the
   replacement publishes and indexes successfully.

Already indexed outputs stay in place. An unindexed legacy image session
without trustworthy session-specific proof stays pending for salvage. This can
also happen if a new clip stops just before the final marker. The existing
index schema, published folder layout, manual capture, and encoded capture
contracts stay as they are. Older outputs already published to recording
folders need a separate audit. The implementation uses additive session
metadata and a source revert for rollback.

## Acceptance

- On the same live A/B/C TGA fixture, recover all 180 frames and the playable
  WAV for A. Preserve B's source frames and WAV byte for byte and keep it out
  of the index. Keep C unrecorded. Repeat recovery and verify media, index,
  unrelated artifacts, and TF2 cfg/custom/display settings.
- Run normal JPG without a WAV and normal TGA with a valid WAV. Verify the
  complete frames and audio independently with FFmpeg where applicable.
  Exercise early TF2 exit while the Toolkit remains open and restart recovery
  after the Toolkit exits.
- Reject evidence from a different session or clip occurrence, including
  repeated candidates, out-of-order markers, and an incomplete final log
  line. Cover a crash before the receipt write with a usable session log.
- In an interrupted replacement fixture, preserve the prior published output
  and its index row. Exercise a failed transfer and a later retry. Export
  reconciliation must skip the unfinished destination, and a completed
  retry must add only the intended record.

## Implementation checks

- A new image batch stores evidence for each capture prefix in a session-owned
  console log. The monitor and startup recovery use an atomic receipt that
  describes the verified artifacts and their SHA-256 hashes. On restart, a
  retained log can supply evidence when TF2 finished a clip before the
  Toolkit wrote the receipt.
- Ralph reviewed the implementation and identified a retry failure caused by
  temporary cross-volume copy files inside the publishable directory. Those
  files now stay in a separate staging folder on the destination volume.
  Their retries use deterministic, session-specific paths.
- All 234 Windows Rust workspace tests passed with Rust 1.88.0. The tests
  cover combined console markers, unrelated and repeated markers, partial log
  lines, transfer retry, and retention of a prior index row after incomplete
  and legacy recovery.
- The live Windows JPG run published 180 decodable 1280×720 frames with no
  WAV. In the interrupted TGA run, the Toolkit monitor was held before its
  receipt write. Startup rebuilt the receipt from the session log, published
  the complete 180-frame sequence and its decodable 2.995-second WAV, and
  preserved the nine partial frames and audio byte for byte. The unstarted
  and partial candidates stayed out of the index. A second restart preserved
  media hashes and index bytes. The isolated TF2 cfg/custom file hashes and
  display registry snapshot matched the pre-run state.
- Normal TGA capture also published all 180 frames and its playable WAV.
  Closing TF2 mid-capture while the Toolkit stayed open published only the
  completed clip, retained the partial media, and restored TF2 settings.
  FFmpeg decoded the complete output in all four native cases.
