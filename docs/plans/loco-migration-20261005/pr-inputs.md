# Preserved pull request inputs

Read from GitHub and fetched on 5 October 2026. Local refs under
`refs/loco-migration/input-pr-<number>` preserve these objects independently of
remote branch deletion. These refs are local; incorporation and acceptance are
tracked below and in the progress ledger. PRs #2397, #2399, #2402 and #2403 were closed as superseded framework
choices after their current heads were verified against these captures and linked
to replacement draft #2451 and delivery issue #2450. Their source branches were
retained. Useful behavior still requires migration and acceptance; closing these
PRs does not claim UI parity. PRs #2419 and #2395 were also closed on 6 October
after replacement commit `3840bded` was published in #2451. Their captured heads
still match GitHub. The schema baseline checksum remains exact, and the Geist
font and licence have identical Git blob hashes in the Loco assets. Source
branches remain available. Scratch, profile and notifications PRs remain open
until their useful work has a published replacement.

Acceptance/disposition remains in the capability inventory and progress ledger.

| PR | Head commit | Retain |
| --- | --- | --- |
| 2419 | 6d607c71c393a79cd259e39f13d48f1977fba1e2 | Complete SQL baseline and schema assertions |
| 2418 | d11483c2dcfaa5e3cdbcaaab8f3f5707512efc0b | Static scratch runtime design |
| 2389 | ad7502b8cd2df9f3a0e6ec46edc55956dc8d61d9 | Notification preferences and journeys |
| 2390 | 4355b2fd47a37e89c0775dbf588de5edca8ceabb | Complete profile/security/advanced behavior |
| 2395 | 1b60dcbf4ab4c60b3f4be1798d939fbe00fd11c4 | Official Geist assets and licence |
| 2397 | 750f347589e4a45f70fa089d66bba546a7046a05 | Existing style/interaction requirements |
| 2399 | 73f3dfa8617728274974e8a37f240eafa5dbdea9 | Useful control behavior; retire legacy framework implementation |
| 2402 | 8bc8e335ac2e0a531182698711f8d4f8b0ce5538 | Profile control behavior |
| 2403 | ac6cdc99d7e92e20433e78d405b94ed5e8a17fa6 | Shared-control behavior; retire Loom dependency |

PR 2381 is unrelated release work and remains outside the migration.
