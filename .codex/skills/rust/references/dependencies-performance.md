# Dependencies and performance

For crate selection, compare the existing implementation with a candidate on
the exact missing capability. Check maintained primary documentation through
Context7, locked versions, supported targets, feature compatibility, licence,
maintenance and dependency impact. Research does not itself require installation.

Prefer a maintained crate for security primitives and standard protocols.
Keep medication eligibility, household permissions and Rails-specific storage
adapters in project code. A general crate cannot supply those product rules.

Before adopting a crate, prove the difficult boundary with a focused case:
persisted Rails compatibility, SSR/client target support, serialisation format
or the required failure behaviour. Document the choice and rejected mismatch
briefly in the relevant change, rather than writing a library survey.

For performance work, identify the user-visible symptom and measure the affected
path in the intended build and workload. Check database round-trips, pagination,
blocking work on async request paths and connection contention before cosmetic
clone/iterator changes. Preserve cancellation and transaction semantics when
moving work between tasks.

Keep memory claims scoped to the measured processes and workload. Historical
API-only idle measurements do not prove the combined web/API budget.
