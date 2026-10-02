# Independent wording-tranche review

Reviewed the shared wording diff, `02-wording-brief.md`, and the writer report on 7 September.
This reviewer did not run tests, a server, or browser automation.

## Requirements verdict: accepted

The implementation covers the accepted wording scope without changing medical rules, authorisation,
submitted enum values, routes, or authentication behaviour. The timing card selects day, week, or
month text through the existing `DoseCycle` normalisation used by enforcement; its missing and
invalid-value fallback remains daily. It now renders singular/plural dose and interval copy across
all five locales. This corrects the safety-relevant weekly/monthly display defect without changing
the rule that determines whether a dose is allowed.

The location count delegates plural selection to I18n instead of deriving a noun from a flash. The
stock-check server render and Stimulus updates agree on zero, singular, and plural labels. Finder,
location warning, schedule workflow, admin labels, invitation options, and anonymous authentication
copy use their new translation-backed wording. Invitation label tests retain the submitted role,
relationship, and access-level values.

The report records a red 47-example run, green 87-example focused run, a further 36-example
follow-up run, locale-tree synchronisation, and a clean diff check. It correctly identifies its
browser evidence as isolated `task test` browser specs, not the shared Playwright task.

## Code-quality verdict: accepted

The diff follows the existing component, I18n, and Stimulus data-value patterns. The period-aware
timing sentence uses the established domain normaliser directly, while policies and models remain
untouched. Translation keys are structurally synchronised across the five supported locale files.
The added tests exercise observable labels and preserve form values rather than coupling to helper
methods. No further changes are requested.
