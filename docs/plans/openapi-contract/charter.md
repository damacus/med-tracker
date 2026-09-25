# OpenAPI implementation charter

User direction, 25 September 2026: pause the previous journey-first Rust port
and implement the current documented API. The old cutover goal remains paused.

## Source and first milestone

`docs/api/openapi.v1.yaml` is authoritative. Documented API behaviour must
work as documented. Inventory its paths, methods, schemas, authentication,
status codes and errors against Rust. An implemented route alone is not proof
of complete behaviour coverage.

Use this isolated worktree, based on last pushed checkpoint `140a1102`, to
preserve the paused uncommitted journey and Leptos work in the original
checkout. Existing work outside the specification is not new acceptance
criteria and must not be counted as specification coverage.

## Ownership and execution

One Sol implementation agent owns API changes, specification-derived tests,
necessary API test tooling, and its coverage/evidence report. The orchestrator
coordinates, reviews, and integrates. No additional implementation or test
agents and no nested delegation. This supersedes the previous parallel-writer
charter for this work.

For each missing documented behaviour, write a contract test from the OpenAPI
specification, demonstrate failure, then make the smallest implementation
change that passes. Reuse existing isolated API acceptance infrastructure.
Use SeaORM for ordinary reads and writes. Raw SQL requires a concrete database
operation that SeaORM cannot express clearly and thorough write tests.

Do not port or inventory Rails tests, change Leptos, alter the specification to
fit implementation, or invent unspecified behaviour. Report specification
ambiguities explicitly instead of importing requirements from Rails source.
Existing schema and fixture tooling may be used to provision isolated data;
they are not an alternative behavioural specification.

For the active 89-operation completion goal, carry forward the user's original
feature-parity objective and choice to preserve existing rules. When OpenAPI
omits a rule, inspect the specific Rails controller, policy or service and
document the existing rule in OpenAPI before deriving tests. This is the stated
default for the unanswered optional global clarification, not a new user reply.
Do not inventory or translate Rails tests. Report conflicts, suspected defects
and genuinely new product or security decisions; do not encode known bugs as
required compatibility. This supersedes the blanket prohibition on consulting
Rails for unspecified semantics above while retaining OpenAPI-first acceptance.

## Evidence and reporting

Record each documented operation and its relevant schema/security/status/error
requirements, implementation location, test evidence, and remaining gaps.
Separate tested implemented coverage from untested implementation. Classify
omissions as implementation gaps, specification ambiguities, or behaviour
outside the specification. Never claim full coverage without evidence.

Run project Task commands. Use explicit workdir on every command and exported
`CONTRACT_TEST_SUBNET` for the existing isolated Compose runner; a Task CLI
variable does not export it. Keep tests and fixture storage project-local.
No commits, pushes, migrations of real data, deployment, or merge by the agent.
The orchestrator owns publication after applicable checks and review.
