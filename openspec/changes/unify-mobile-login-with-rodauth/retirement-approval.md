# Legacy mobile authentication removal awaiting approval

Automatic approval review rejected this part of the implementation. No removal
from this document has been applied. The new Rodauth mobile flow and configurable
lifetime changes are separate working-tree changes.

The proposed removal is limited to these old first-party endpoints:

| Endpoint | Replacement |
| --- | --- |
| `POST /api/v1/auth/login` | Local login in the Rodauth browser journey |
| `POST /api/v1/auth/oidc_exchange` | Rodauth authorization code redemption at `/token` |
| `POST /api/v1/auth/select_household` | Household navigation using the existing account token |
| `POST /api/v1/auth/refresh` | Rodauth refresh-token grant at `/token` |

Remove the corresponding methods and private helpers from
`Api::V1::Auth::SessionsController`. Preserve household listing, device session
listing, individual revocation and logout. Retire the unused native ID-token
exchange and selection-grant implementation after checking their references.

Update capabilities and the root/pinned OpenAPI contracts to stop advertising
those endpoints. Remove Android's unused exchange/password adapters and their
generation configuration. Existing API/MCP app tokens and SMART/FHIR OAuth
credentials keep their restrictions; their endpoints are outside this removal.

Replace tests of retired endpoints with route-retirement tests and the new
Rodauth request tests. The shared API fixture helper must issue a restricted
test session directly, so unrelated API permission tests do not depend on an
endpoint that no longer exists. Preserve tests for household restrictions,
lockout, expiry, audit redaction and revocation on the replacement paths.

The user stated there are no active mobile clients. If an old client does exist,
these four removed endpoints will no longer work; it must use the new client.
This is the intended direct replacement, with no compatibility period.

The review rejection described the combined route, controller, helper and test
removal as too broad and insufficiently authorised. Approval is needed to apply
this explicit scope. Deployment and merging are not included.
