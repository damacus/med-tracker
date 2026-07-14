# API contract conventions

The OpenAPI v1 contract is defined in [openapi.v1.yaml](openapi.v1.yaml).

## Canonical addressing

The document's first server URL is `/api/v1`. Path keys are relative to that
server and must begin with `/` without repeating `/api/v1`. For example, the
`/households/{household_id}/people` path key resolves to
`/api/v1/households/{household_id}/people`.

## Operation IDs

Every HTTP operation must declare a unique, stable lower-camel-case
`operationId`. Use an action-aware verb: `list` for collection reads, `get`
for singular reads, `create`, `update`, `replace`, or `delete` for CRUD, and a
precise verb such as `pause`, `resume`, `adjust`, `test`, or `dryRun` for a
custom action. PATCH and PUT operations on the same resource need distinct
IDs.

## Tags

Every operation has exactly two tags: one audience tag and one resource tag.
The audience tag is one of `Public`, `Account`, `Household`, or `Household
administration`; the resource tag identifies the narrowest responsible API
domain. Define each used tag once in the top-level `tags` array with a concise
description, and do not define tags that no operation uses.
