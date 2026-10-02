# Wording tranche brief

Issue: #2123. Start only after card tranche review passes. Same Luna writer owns the changes.
Read `scout-review.md` and the translate skill. Keep all five locale trees synchronised.

## Accepted outcomes

- Timing restrictions use plural-aware daily/weekly/monthly copy matching the actual selected
  period: “Maximum 1 dose per week” / “Maximum N doses per week”, and equivalent day/month forms.
  Select through `DoseCycle.new(person_medication.dose_cycle).to_s`, matching enforcement's
  normalisation/default. Use full translated sentences per period, not concatenated fragments.
  This corrects a confirmed display bug: weekly/monthly limits currently say “per day”.
- Waiting intervals use “1 hour” / “N hours”, retaining the existing hours value and meaning.
- Stock-check button uses “Apply 1 amendment” / “Apply N amendments”, including JavaScript updates
  and server-rendered validation states. Preserve zero/many behaviour.
- Schedule workflow uses “Medication”, “Frequency”, and “Schedule summary” instead of
  “Name of med”, “Dose, frequency”, and “Schedule (break this down)”. These fields represent a
  medication selector, frequency text and summary, respectively; no dosing semantics change.
- Location medication counts use a dedicated count translation, never a noun derived from a flash.
  Low-stock badges use the existing translated stock warning where possible.
- Finder package labels reuse the existing translated package label.
- Admin user headings/actions and invitation role/relationship/access labels and options use
  explicit translations, retaining exact underlying submitted enum values and permissions.
- Verification resend uses “Resend verification email”; resend and reset-request footers each use
  one “Back to sign in” link instead of combining back-navigation text with “Welcome back”.

## Ownership

Own `config/locales/*.yml`, affected location index/show, finder, stock-check, schedule workflow,
admin user/invitation components and stock-check/finder Stimulus controllers, plus their existing
component/system/request tests. Also own the existing Rodauth resend/reset-request views for the
copy-only fixes and person-medication timing status for the period-aware sentence. Other behaviour
is read-only. No broad terminology rewrite.
Do not add/remove comments. No changes to medical rules, auth, schema or public API.

## Verification

Add failing assertions before production changes. Cover one/many counts across supported locales,
daily/weekly/monthly limits including default-to-daily normalisation,
one/four-hour waiting labels,
stock-check live 0/1/2 transitions and server error re-render, submitted invitation option values,
and local rendered copy. Use established browser locale setup where available; do not claim browser
locale coverage from a component test. Run focused specs, locale-tree checker, and diff check.
Report exact commands/results and limitations in `02-wording-report.md`; reviewer writes
`02-wording-review.md`. No commits/pushes by writer.

Baseline locale-tree check failed only because English lacks the existing accompanying-locale
pagination keys `admin.carer_relationships.index.pagination.label`, `next`, and `previous`.
Restore those three English labels as part of this wording tranche; preserve the other locales.
