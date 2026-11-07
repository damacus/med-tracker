# Android OpenAPI generated-client provenance

- Pinned OpenAPI source: docs/api/openapi.v1.yaml
- Source revision: 33cafe64a33e27e6170a1dbc3cfd6ce4c49f3f81
- Pinned OpenAPI SHA-256: 2b8d74916af46df7318da778ca59b70cf9b7c4dbf3286aeb2b23f133ac1aa90a
- Derived generator-input SHA-256: 30b234b883c57746864d812ff33d9450484c4e3332bcdf2584bf44dae0a79267
- Generator input: -i /output/openapi-generator-input.yaml, derived from the exact canonical pin
- Generator-input adapter: mobile/android/scripts/api-generator-input.fish
- Single transformed schema: #/components/schemas/PortableImportUnprocessableResponse
- Transformation: closed oneOf becomes the previous flat optional typed data/error fields for Kotlin okhttp4/Moshi generation only
- Validation limitation: Android retains its previous permissive decoder; the canonical server and Swift contract remain strict. The derived input is not validation-equivalent or byte-identical to the canonical source.
- Generator image: openapitools/openapi-generator-cli:v7.20.0@sha256:fa4add01856e44becf70674164df354d61bd37ba0f444d27be949801e013921b
- Generator: OpenAPI Generator 7.20.0
- Release generator configuration: mobile/android/openapi-generator-config.yaml plus openapi-generator-release.ignore
- Release-safe generated Kotlin artifacts: mobile/android/phone/src/main/kotlin/io/medtracker/client
