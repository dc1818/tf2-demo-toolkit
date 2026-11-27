# TF2 events.txt bookmark import

A16 imports explicit TF2 Demo Support bookmarks from `_events.txt`
or `events.txt` beside a selected demo. A batch reads its shared logs
once and resolves references against its selected demo paths. The
recorded tick is a demo tick. The log timestamp is metadata, so its
date and timezone have no effect on clip timing. [Valve Demo Support
release notes](https://www.teamfortress.com/post.php?id=22132)

## Demo matching and bookmark behavior

- A bare title or filename identifies a selected demo in the log's
  directory. Relative paths resolve against the log and explicit paths
  match a selected demo directly. Slashes, case, and the optional
  `.dem` extension are normalized for matching. References with stale
  directories and ambiguous matches are reported. The importer reads
  only the two known log filenames beside selected inputs.
- TF2's `Bookmark` rows supply the tick and comment. The parser accepts
  UTF-8 and BOM-marked UTF-16, including CRLF files. Automatic
  `Killstreak` rows are counted separately, consistent with the JSON
  bookmark reader. Text is handled as data.
- A bookmark with the same demo, tick, and trimmed case-insensitive
  comment as an earlier mark is counted as a duplicate. Existing JSON
  and embedded marks take precedence. New distinct text bookmarks use
  the existing bookmark attachment and score rules. Standalone text
  marks derive a stable ID from their tick and normalized comment.
  Existing standalone IDs keep their earlier numbering.
- The final `frag_summary.json` carries `events_txt_import` for a
  single demo or `events_txt_imports` for a combined batch. Reports
  include matches, imports, duplicates, ticks beyond the demo, ignored
  killstreaks, malformed records, unmatched and ambiguous references,
  and log read errors. The recorded rejection line numbers refer to
  the source log. Source demos and bookmark files are opened read-only.

## Validation fixture

The player-published Habib i63 grand-finals archive has one shared
`_events.txt` with three explicit bookmarks over five demos and 34
automatic killstreaks. Each explicit bookmark also appears in an
accompanying demo JSON sidecar. The log SHA-256 is
`9f7cae216361e287c5ac9797523905daa3464f149ce1a4627c6747fe577e3420`.
[Player's fixture post](https://www.teamfortress.tv/49305/i63-pov-demos-thread/#18)

The native catalog assigned all three marks to the intended demos.
A Windows batch on the Process POV imported the text-only bookmark
at demo tick 46188 and applied the existing 30-point bookmark rule
once. The clip `r2-p79-t97442` had a final score of 58. The native
JSON-only and text-plus-JSON runs produced identical sets of 71
candidates. The dual-input report recorded one duplicate and zero
new imports.

The final Windows analysis and combine replay reused the files
created by the real batch parser. It preserved the candidate output
and retained the import report in the final summary. After normalizing
source paths and batch metadata, the text-only and standalone JSON
results matched except for round-trip precision on three recorded
duration fields, with a maximum absolute difference below
`0.000000000000008` seconds. Source bytes matched the original archive.

All 244 Windows Rust workspace tests passed. The new regressions
cover explicit paths, same-named and ambiguous selections, stale
paths, UTF-16, malformed and out-of-range rows, JSON/text duplicates,
legacy bookmark IDs, and single- and multi-demo summary diagnostics.
