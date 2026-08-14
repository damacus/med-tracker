# API versioning and client policy

The checked-in OpenAPI document is the source of truth for the MedTracker product API. First-party mobile applications, the CLI, and local automation use `/api/v1`.

This policy applies to the HTTP contract. Database models, Rails classes, and web routes are not public API contracts.

## Change classes

### Additive changes

An additive change adds an optional request field, response field, operation, enum capability, or error detail without changing existing behaviour.

Clients must ignore response fields they do not understand. They must not send a new request field until their target server advertises or documents support for it.

### Compatible changes

A compatible change preserves every existing valid request and its documented meaning. It can clarify descriptions or widen a value constraint. It can also add an optional field or operation.

A new error response is compatible only when the server could already reject that request for the same reason. The error code and recovery advice must remain stable after publication.

### Deprecated changes

A deprecated operation or field remains usable during its notice period. The OpenAPI document marks it as deprecated and names its replacement.

MedTracker keeps deprecated `/api/v1` behaviour for at least two minor releases and 90 days after the first released replacement. Security or data-safety defects can shorten this period. The release notes must explain any exception.

### Breaking changes

A change is breaking when an existing conforming client must change to keep working. Examples include:

- removing or renaming an operation, field, schema, enum value, or error code;
- making an optional request field required;
- narrowing an accepted value or changing its meaning;
- changing authentication, authorization, pagination, or idempotency behaviour;
- changing an operation ID or reusable schema name;
- changing a successful response to a different shape or status.

Breaking product API changes require a new major path such as `/api/v2`. Do not place them silently in `/api/v1`.

## Stable names

Published `operationId` values are stable client method names. Reusable schema names are stable generated type names. Do not rename either inside the same major API version.

If wording improves, change the summary or description. Keep the operation ID and schema name unchanged.

## Releases

Every release that changes the OpenAPI document must classify the change in its release notes. The pull request must update the contract before or with the Rails code.

The compatibility gate compares the proposed OpenAPI document with the released baseline. It must fail for an unapproved breaking change.

Deprecation release notes include:

- the deprecated operation or field;
- the supported replacement;
- the first deprecated release;
- the earliest removal release and date.

## Generated clients

Generate client request types, response types, and operation methods from `docs/api/openapi.v1.yaml`. Do not copy JSON examples into application code as a second contract.

Generated files must record the OpenAPI revision used to create them. Regenerate them when the contract changes, and review the generated diff with the OpenAPI diff.

Keep handwritten code at the product boundary. Authentication storage, retries, command wording, user prompts, and domain workflows remain maintained client code. Generated code owns only HTTP transport types and operation bindings.

The current Rust tools predate a checked-in generator. Until generated bindings replace their handwritten transport types, the OpenAPI contract and compatibility gate remain authoritative. New client surfaces must not add another handwritten copy of request or response schemas.

## Runtime capability checks

Clients use `GET /api/v1/capabilities` before relying on optional server features. Capability checks do not replace version compatibility. They report deployed features. The OpenAPI document defines their payload and meaning.

Clients must handle an older compatible server that lacks a newly added capability. They must fail with a clear unsupported-server message instead of sending an unadvertised request.
