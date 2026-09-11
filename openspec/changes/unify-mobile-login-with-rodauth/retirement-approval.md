# Approved legacy mobile authentication removal

The user explicitly approved this removal and reaffirmed that there are no
active clients. The four endpoints and their client transports are now removed.
The earlier automatic approval block is resolved for this scope.

The removal covers these old first-party endpoints:

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

Deployment and merging are not included. Existing release packaging validation
remains in place; removing that validation was separately rejected by automatic
review and is not needed for endpoint retirement.
