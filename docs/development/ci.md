# Required CI checks

Pull requests run `Classify Changes` and finish with `CI Success`. The main
ruleset requires `CI Success` and `Validate PR title`, rather than individual
conditional jobs or matrix shard names.

| Changed inputs | Mandatory validation |
| --- | --- |
| Rails implementation, dependencies and runtime configuration | Security scans, RuboCop, RSpec including both browser shards, production-image smoke |
| Android implementation and build configuration | Android tests, lint, assemblies, release checks and pinned API drift |
| Published documentation and site configuration | Documentation build |
| Markdown and plans | Changed-file Markdown syntax and whitespace |
| Rust client tools | Formatting, compilation, Clippy and tests |
| OpenAPI and native generator inputs | Deterministic generation and Kotlin/Swift compilation |
| CI workflows | Workflow syntax validation |

The root OpenAPI contract also selects Android verification when the Android
application is present. Native applications use pinned contract copies.
An Android README alone runs Markdown validation. A plan inside `docs/` also
runs the site build because it is published documentation.

Lighthouse and mutation testing produce advisory reports. Their results and
duration do not block the required gate. Live canary tests, releases and
deployments are separate workflows.

## Routing and failures

`scripts/ci/policy.json` records path ownership and mandatory jobs. Changes to
the classifier, gate, policy or shared root tooling select every implemented
suite. Mixed changes select the union of affected suites. An unknown path
fails classification until its ownership is added explicitly.

PR classification compares the current merge commit with the actual PR base,
including stacked branches. Push classification compares the before and after
commits. Deletions and both sides of renames are included without API file-list
limits.

Every selected mandatory job must report success. Failure, cancellation,
unexpected skipping and missing results fail the gate. Reusable workflows also
expose completion markers from their validation jobs, so a skipped child job
cannot appear as successful validation. Each run includes its path selections
and gate result in the job summaries.

## Local checks

Run `task ci:check` with actionlint 1.7.12 installed to validate workflows and
CI scripts. Run `task ci:classify` or `task ci:markdown` with `CI_BASE` set to the
base commit; `CI_HEAD` defaults to `HEAD` and `CI_EVENT` defaults to PR comparison.
Set `CI_EVENT=push` for a before/after comparison. Run `task ci:gate` with
`CI_NEEDS` containing GitHub's JSON `needs` object.

Temporary routing, gate and deliberate-failure tests are removed after pipeline
verification. GitHub run links retain the verification evidence.
