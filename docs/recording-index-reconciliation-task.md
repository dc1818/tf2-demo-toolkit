# V04 follow-up for unresolved recording paths

**Ralph follow-up.** The source review started at pilot checkpoint `d79b0d8`.
The Windows reproductions used the same production code at `3091563` and
disposable test data.

## Evidence

- A read-only audit found 124 clips in the configured Videos folders. Of 242
  latest recording keys, 109 referenced existing paths and 133 referenced
  paths that did not resolve. The count of 178 also included paths referenced by
  historical index rows.
- Comparing all 124 files against the unresolved entries found no exact
  SHA-256 or generated recording-key token match. Two unresolved entries had
  no saved fingerprint. The audit covered the configured video library.
- The recorded-status lookup checks that an indexed output still exists.
  Candidate selection and replacement lookup use that same check. Export load
  separately tries to reconcile recordings in the configured output root.
- The initial Ralph source review identified two disk-write cases to
  reproduce. Fingerprint relocation can append a byte-identical output with
  another recording's generated token, even though the status lookup later
  rejects that token. A background reconciliation can also append from its
  snapshot before the UI checks that its export and output root are current.

## Acceptance checks

1. Load candidates with one valid indexed clip and one unavailable indexed clip.
   Verify the Recorded column, the prompt count, and the queue after selecting No.
   Verify that the valid clip stays in its category folder and the unavailable
   path is ineligible for replacement.
2. Move and rename a disposable clip within the supported output layout, then
   load the export. Verify that a unique exact match restores the Recorded
   state and persists the relocated path. Reopen the index and repeat the
   load to check that reconciliation does not add redundant rows.
3. Give a different recording a foreign generated token and byte-identical
   media. Confirm that its path is never persisted for the missing original.
   Pause reconciliation after its index snapshot, persist a newer valid
   entry, then resume it. Confirm that a fresh reload still resolves to the
   newer entry. Use index contents on disk for both assertions.
4. Repeat with an unavailable output drive, a later reconnect, a same-name
   different clip, and a file that disappears while the confirmation dialog is
   open. Check that an unresolved or ambiguous entry never skips the wrong
   candidate or authorizes a replacement. Verify recording history and the
   original fixture output paths and hashes after the run.
5. If a fixture reproduces a defect, record the failing case and the smallest
   compatible fix in the implementation register. A change to association
   rules, on-disk history, output layout, or UI follows the guide's approval
   protocol.

## Windows reproduction results

- A video carrying B's generated token had the same SHA-256 as missing
  recording A. Fingerprint relocation appended B's path for A. Reopening the
  index showed the incorrect association, while A's Recorded flag stayed
  false and its replacement-target list stayed empty. B's path and bytes
  were preserved.
- Both name discovery and fingerprint relocation could supersede a newer
  valid entry. A deterministic fixture cloned the index, registered a newer
  valid recording through the normal writer, then resumed the stale scan.
  The in-memory merge retained the newer path. Reloading from disk chose the
  older path because the scanner had appended it after the newer row.
- An unavailable fixture root stayed out of the Recorded and replacement
  results. No queued the unavailable candidate and skipped the valid one.
  Restoring the root restored the original paths without appending a row.
  A same-name video with different bytes stayed unassociated. A uniquely
  matching renamed copy was persisted once and survived a reload.
- An offscreen Toolkit fixture invoked the production export-load and
  Record Selected callbacks with two recorded candidates. The fixture moved
  one disposable MP4 at the prompt boundary and supplied No. The handler
  reported that both candidates already had matching recordings and exited
  before preflight, although a fresh status calculation found one missing
  recording. Both rows stayed selected. The fixture retained all MP4 bytes
  and the complete index, and no game or capture process launched.

The Windows index probe produced three expected failures and two compatibility
passes. All eight existing recording-reliability tests passed. The disk tests
used small synthetic media files to exercise identity, persistence, and queue
selection. Ralph reviewed the fixture design and the proposed commit boundary.

## V04 reconciliation proposal v1

**Approval pending.** The scope covers V02/V03/V04 and the reproduced failures.

1. Reject a recognized foreign recording token before fingerprint relocation.
   When several eligible outputs share a fingerprint, leave the association
   unresolved. A uniquely matching renamed file without a token can still be
   linked, and existing generated-name matching remains available.
2. Stage discoveries in the background. Commit them only for the still-current
   export and output root. Compare the latest on-disk index entry with the
   entry used in the scan and append only if the association is still
   unresolved and the entry has not been superseded. Serialize that comparison
   and append with the index's recording and recovery writers. Keep scanning
   and hashing outside the writer lock.
3. Recheck the selected candidates' Recorded status when the native prompt
   returns. If the recorded set changed, refresh the table and cancel the
   pending start. The user keeps the selection and can run Record Selected
   again with the current status. Use the existing cancellation status and
   prompt. Stable selections retain the current Yes, No, and Cancel behavior.

Keep the append-only index schema, output folders, candidate identity rules,
legacy tokenless discovery, and existing recording history. Retain unresolved
references so they remain available for reconciliation if the corresponding
media become accessible. Rollback is a source revert. Successful relinks use
normal index rows that the previous build can read.

Acceptance requires the three failing disk probes to pass, coverage of
ambiguous matching and outdated export/root context, and verification that a
file change after the prompt refreshes the Recorded column without starting
a batch. Repeat the prompt case with the same count but different Recorded
rows. The legacy, same-name, reconnect, failed-write, and successful relocation
cases must continue to pass.
