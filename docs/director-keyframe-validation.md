# Director keyframe synchronization after restart (V09 / D01)

November 14, 2026. Baseline `0349aa4d70e841a19097298217f34be863879e56`.
Windows x64 TF2 and HLAE 2.189.4. An isolated Toolkit and game harness used
the manual Director launch path, public SourceTV demo 1530633, and three
in-clip HLAE keys. [Demo](https://demos.tf/1530633)

## Reproduction

The fixture saved the HLAE campath before Safe Restart, after reload, and
after a backward seek. All three baseline XML files were byte-for-byte
identical and contained three points. The live timeline and cue card both
showed only IDs 0 and 1 at ticks 6013 and 6028 after the seek. A corrupted
third console row ended in `Y n 2 : ----`. The parser treated the line
as a separator and published the two rows it had collected.

Other console snapshots joined the tick and formatted demo clock into one
token, such as `601301m30s` and `01m30s6028`. HLAE prints those fields
in separate calls. The Toolkit could read neither joined token as a tick.
[HLAE 2.189.4 print implementation](https://github.com/advancedfx/advancedfx/blob/v2.189.4/shared/MirvCampath.cpp)

## Repair

The parser rejects a table if a row has no valid tick. The last verified
keyframe model stays visible and the existing refresh path can re-query with
a shared two-retry limit for incomplete and transient empty tables. It also
recognizes the two observed joins when a sub-hour clock has an unambiguous
boundary, including a displaced leading comma. A begin marker may share its
line with the table header. Merged rows, missing or repeated IDs, and other
malformed forms trigger rejection. HLAE retains authority over camera data
and key IDs. Both widgets use the same verified model and retain the
existing clip bounds.

## Verification

- The exact truncated-row regression failed against the baseline parser.
  All three added regressions and the locked Windows Rust workspace suite
  passed with the repair (216 tests).
- The repaired live run rejected an incomplete print before restart. HLAE
  saved identical three-point XML before restart, after restart, and after
  the backward seek. The saved times, coordinates, rotations and FOV were
  unchanged.
- After Safe Restart, both live models held all three IDs. After the
  67-tick backward seek and refresh, screenshots and model diagnostics
  confirmed IDs 0, 1 and 2 once each at ticks 6013, 6028 and 6046.
  Two intermediate approximate tick values differed by one tick in HLAE's
  print. The Toolkit uses the latest verified estimate.

The test covers one public STV clip and HLAE build. It leaves the broader
D01 matrix open, including import failures and other runtime conditions.
D03's timeline policy for keys outside the clip remains a separate decision.
