# Android OpenAPI generated-client provenance

- Pinned OpenAPI source: docs/api/openapi.v1.yaml
- Source revision: 1b81a61e61340ea4c3afe12ebcbcf0f7a93c033e
- Pinned OpenAPI SHA-256: 34fb95fd9b219f1d35f206f6a3e924cbd3a9892b149484b58656c0261a39247b
- Derived generator-input SHA-256: 18431095ad36d309823912fee10a4cb9004293b484bd500d7cd4078ab9be7cbc
- Generator input: -i /output/openapi-generator-input.yaml, derived from the exact canonical pin
- Generator-input adapter: mobile/android/scripts/api-generator-input.fish
- Single transformed schema: #/components/schemas/PortableImportUnprocessableResponse
- Transformation: closed oneOf becomes the previous flat optional typed data/error fields for Kotlin okhttp4/Moshi generation only
- Validation limitation: Android retains its previous permissive decoder; the canonical server and Swift contract remain strict. The derived input is not validation-equivalent or byte-identical to the canonical source.
- Generator image: openapitools/openapi-generator-cli:v7.20.0@sha256:fa4add01856e44becf70674164df354d61bd37ba0f444d27be949801e013921b
- Generator: OpenAPI Generator 7.20.0
- Release generator configuration: mobile/android/openapi-generator-config.yaml plus openapi-generator-release.ignore
- Release-safe generated Kotlin artifacts: mobile/android/phone/src/main/kotlin/io/medtracker/client
