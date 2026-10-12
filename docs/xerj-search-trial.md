# Xerj source-search trial

On 12 October 2026, Xerj 1.0.0-rc.95 was compared with ripgrep against the
application's `src/` tree at commit `449fb5107`. The release archive's SHA-256
matched its published checksum. The binary and index were kept in temporary
storage, with the server bound to localhost. No repository integration or
external embeddings were configured.

## Setup and scope

Xerj used its default lexical embeddings, the code analyzer, two workers and
no relationship graph. Indexing completed in 27.1 seconds: 332 files, including
all 320 code files, produced 3,163 records with no junked code files.

Each search was run once, sequentially, against the same source tree. Times
include client process startup. Xerj returned its default five ranked hits as
JSON; ripgrep returned all matching lines. The questions and regexes deliberately
represent each tool's normal interface; this is a small workflow trial, not a
controlled search-quality or token-cost benchmark.

| Task | Xerj question | ripgrep pattern | Xerj / rg time |
| --- | --- | --- | --- |
| Exact symbol | `verify_membership` | `verify_membership` | 274 / 119 ms |
| Membership boundary | `where is household membership verified before setting database tenant context` | `verify_membership\|current_household_id` | 155 / 24 ms |
| Audit construction | `actor membership request credential provenance audit context` | `actor_account_id.*actor_membership_id` | 94 / 28 ms |
| Response ETags | `canonical JSON sorting hashing response etag` | `sort_all_objects\|Sha256::digest` | 104 / 26 ms |
| Household references | `HouseholdId` | `HouseholdId` | 142 / 27 ms |

## Findings

- Xerj ranked `models/access.rs` first for both the symbol and membership
  question. This makes it useful when the caller does not yet know a symbol.
- The audit query found dose audit construction but omitted the location,
  people and administration context builders from its first five results.
- The ETag query found relevant sync and occurrence files, but also unrelated
  identity canonicalisation. It omitted the location and people representations.
- The household query returned five ranked hits. ripgrep returned 311 matching
  lines, which is more useful when checking the complete replacement scope.
- ripgrep was faster in every pair. Xerj returned more JSON bytes in the first
  four pairs; only the household-reference search was smaller. These byte counts
  do not establish model token use or total task cost.

## Decision

Keep ripgrep as the primary tool for exact searches, exhaustive refactoring
inventories and confirming no old patterns remain. Xerj can supplement it for
plain-English discovery, but this trial does not justify replacing ripgrep or
adding a permanent service. Neural embeddings, ongoing index freshness and
larger source/document corpora were not evaluated.

The implementation refactors should proceed independently of Xerj adoption.
