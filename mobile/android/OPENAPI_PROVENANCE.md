# Android OpenAPI generated-client provenance

- Pinned OpenAPI source: docs/api/openapi.v1.yaml
- Source revision: 28c913937d44823114623eb33afa9972f580281b
- Pinned OpenAPI SHA-256: fbbb0f8cb7c842f9bd24f324a65f929269cb3f895de3ddec06a15459126d79c7
- Derived generator-input SHA-256: b7a0cb000912ccc4e178c8c83f0cf2597c6b27e7725818abb6a090e9489be2ed
- Generator input: -i /output/openapi-generator-input.yaml, derived from the exact canonical pin
- Generator-input adapter: mobile/android/scripts/api-generator-input.fish
- Single transformed schema: #/components/schemas/PortableImportUnprocessableResponse
- Transformation: closed oneOf becomes the previous flat optional typed data/error fields for Kotlin okhttp4/Moshi generation only
- Validation limitation: Android retains its previous permissive decoder; the canonical server and Swift contract remain strict. The derived input is not validation-equivalent or byte-identical to the canonical source.
- Generator image: openapitools/openapi-generator-cli:v7.20.0@sha256:fa4add01856e44becf70674164df354d61bd37ba0f444d27be949801e013921b
- Generator: OpenAPI Generator 7.20.0
- Release generator configuration: mobile/android/openapi-generator-config.yaml plus openapi-generator-release.ignore
- Release-safe generated Kotlin artifacts: mobile/android/phone/src/main/kotlin/io/medtracker/client
