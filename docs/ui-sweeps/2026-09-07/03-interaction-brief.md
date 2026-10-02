# Interaction and sweep completion brief

Issue: #2123. Start only after wording review passes; retain the same sole writer.

## Confirmed fixes

- INV-03: make location member removal visibly discoverable without hover. Preserve its accessible
  name, confirmation dialog and focus return. Test keyboard activation/cancel at 390px.
- INV-04: keep profile tab labels within their controls at 320px and enlarged text, with working
  keyboard navigation. Prefer wrapping, auto-height tabs over hiding text. Preserve tab semantics.
- ADM-05: use the existing import policy capability at the dashboard controller/query/component
  boundary. Household managers without platform permission must see neither the import action nor
  non-actionable import attention counts. Platform admins retain both; direct-route denial remains.
- Auth flash duplication: retain one visible inline auth-form alert, suppress duplicate global
  feedback on the same page. Verify logout and invalid-login feedback appears exactly once, while
  ordinary authenticated pages still show global notices. Preserve error announcements.

## Remaining reproduction candidates

Follow `scout-review.md` for finder long metadata, mobile-rail long shortcuts and invitation long
emails. Write realistic failing geometry tests before any fix; if no failure, retain tests and mark
the candidate rejected with evidence. No speculative production edits.

## Ownership

Own location show, profile tabs, dashboard controller/metrics query/view and their tests. Also own
finder result layout, mobile rail, invitation row layout only after reproduction. Preserve policies,
submitted data and global data boundaries. Own existing mobile audit/overflow specs and focused
support helpers to finish the matrix. Shared component library remains read-only unless escalated.
Existing application layout and auth flash rendering are owned solely for the duplicate-alert fix.
Do not add/remove comments.

## Sweep completion and verification

Expand the existing route audit to light/dark at 390/1280, including stock check, medicine reviews,
health history, household settings and available signed-out forms omitted from the earlier matrix.
Do not count redirects to login/unauthorised as successful page coverage. Keep representative role
journeys and add meaningful role checks wherever visibility changes. Stress shared components at
320px, tablet width and 200% text size. Report unsupported stateful authentication steps explicitly.

Run targeted red/green specs and save before/after local screenshots under `docs/screenshots/`.
Record commands, route/state coverage, remaining limits and screenshot paths in
`03-interaction-report.md`. Independent review goes in `03-interaction-review.md`.
Coordinator runs the final combined gates and obtains a final broad review before publication.
