# Android OpenAPI generated-client provenance

- Pinned OpenAPI source: docs/api/openapi.v1.yaml
- Source revision: f1699def48741d1030b9c82e1e57fbdd4af42e95
- Pinned OpenAPI SHA-256: 38ebde30cf5704221ddf6f9546cc4d0d3b9a1aa0b4e539dae28e24045564826b
- Derived generator-input SHA-256: 3014a6e7e441131fbef4fbcf35996127436a6dd82d99344a43fd60b635dbf851
- Generator input: -i /output/openapi-generator-input.yaml, derived from the exact canonical pin
- Generator-input adapter: mobile/android/scripts/api-generator-input.fish
- Single transformed schema: #/components/schemas/PortableImportUnprocessableResponse
- Transformation: closed oneOf becomes the previous flat optional typed data/error fields for Kotlin okhttp4/Moshi generation only
- Validation limitation: Android retains its previous permissive decoder; the canonical server and Swift contract remain strict. The derived input is not validation-equivalent or byte-identical to the canonical source.
- Generator image: openapitools/openapi-generator-cli:v7.20.0@sha256:fa4add01856e44becf70674164df354d61bd37ba0f444d27be949801e013921b
- Generator: OpenAPI Generator 7.20.0
- Release generator configuration: mobile/android/openapi-generator-config.yaml plus openapi-generator-release.ignore
- Release-safe generated Kotlin artifacts: mobile/android/phone/src/main/kotlin/io/medtracker/client
