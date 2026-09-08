# Remaining long-content candidates

Nightingale owns this small verification step after the route audit. Hubble reviews the results.
The finder candidate is already reproduced and fixed in the interaction tranche; do not redo it.

Use the existing mobile overflow spec and established shortcut/invitation fixtures or helpers.
Only these two candidate cases are in scope:

1. Three eligible long-labelled mobile shortcuts at 320/390px, including English, Portuguese and
   Welsh labels. Check text containment and usable control height as well as page overflow.
2. A syntactically valid invitation email with a 64-character local part at 320/390px. Check that
   text stays within its row and actions remain reachable, not just that the page cannot scroll.

First write and run the realistic regression cases without production edits. If they pass, record
the candidates as rejected for the tested data. If either fails, send exact geometry and the
smallest proposed production correction to the coordinator for scope assignment; retain sole
writer ownership and do not broaden the change.

Run the affected file only through the isolated `task test TEST_FILE=...` route. Record outcomes,
viewport/locale/data boundaries and any correction in `05-long-content-report.md`, then return.
Do not add speculative fixes or a general terminology rewrite.
