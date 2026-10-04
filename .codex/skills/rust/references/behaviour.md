# Behaviour and access

Use Rails observable behaviour, the root OpenAPI document and the relevant
OpenSpec change together. Record deliberate corrections to Rails defects;
do not silently copy a bug or redefine a public response.

Browser handlers must reuse the authorised application/API boundary already
used by nearby routes. Do not add component database reads, an HTTP call back
into the same process or an alternative browser permission policy.

Before changing access, trace the credential type through session or token
validation, household binding and person visibility. Test hidden related IDs
and aggregate counts as well as the primary record. Authentication alone does
not establish permission to a submitted person, location or medication.

For browser writes, retain the existing CSRF and session lifecycle. For mobile
and integration writes, retain their separate credential and scope semantics.
Do not assume one credential type's successful test covers the others.

For Rodauth coexistence, verify persisted byte formats, key derivation, cookie
and challenge behaviour against Rails fixtures. A crate's successful round-trip
only proves its own format. Use maintained security primitives, with a narrow
compatibility adapter where the stored Rails format requires one.

Review externally visible errors for health-data and credential disclosure.
Keep detailed diagnostic context at the existing private logging boundary.
