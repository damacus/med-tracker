# Enlarged text correction

The final screenshot inspection exposed failures in the planned 200% text-size check. This is a
narrow correction to the approved sweep, not a redesign. Nightingale remains the sole writer.
Hubble reviews shared navigation/notice containment; Sol reviews the profile tab layout judgment.

## Red evidence first

At 320px with the document root font set to 32px, record computed rectangles, font sizes and text
ranges for the mobile menu/search controls, brand, bottom-rail labels, warning description and
dismiss control. Do not infer fixed pixel sizes from rem utilities: `h-20` scales with root font.
Add failing regression assertions for the demonstrated clipping/overlap before production changes.
Keep ordinary 16px-root behaviour covered.

The profile screenshot also shows four thin columns with words broken into many fragments rather
than the intended readable wrapping layout. Measure its actual computed display/flex/grid values
and row/column bounds. Require readable full labels and a deliberate narrow layout, not merely
text ranges squeezed into increasingly tall controls. Preserve all accepted keyboard and link-filter
semantics. Await the coordinator's scoped layout decision after Sol's review.

## Owned paths after diagnosis

- `app/components/layouts/navigation.rb`
- `app/components/layouts/mobile_rail.rb`
- `app/components/layouts/flash.rb`
- `app/views/profiles/show.rb`
- Existing focused mobile-overflow/profile specs, or one small dedicated enlarged-text system spec.

Change only demonstrated min-width/wrapping/layout constraints. No font-size reduction to defeat
the enlargement check. Preserve notice text, timing and dismissal, accessible brand/link names,
menu/search controls, navigation destinations and permissions. Shared RubyUI and application-shell
padding are read-only unless measured evidence requires a separate coordinator decision.

## Verification and recapture

Run focused tests first. Capture a readable final enlarged profile view after the notice has fully
dismissed, plus a shell/notice screenshot demonstrating the repaired bounds. The existing
`enlarged-text-320-before.png` is before evidence, not a fixed screenshot. Do not call a faded
overlay over tab text a successful capture. Remove temporary capture specs and correct the 06 report
only after reviewing the actual images.

Record red/green commands, geometry, changes and artifact paths in `07-enlarged-text-report.md`.
Return at each requested boundary; no further audit expansion or publication by the writer.
