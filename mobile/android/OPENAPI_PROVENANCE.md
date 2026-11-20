# Android OpenAPI generated-client provenance

- Pinned OpenAPI source: docs/api/openapi.v1.yaml
- Source revision: 4681b514225086f94555472e31c6bf71ed0de101
- Pinned OpenAPI SHA-256: b79b265ca2a75966464b55e2883f13d0527ea1ad74da58261327d1e1d1a23cd8
- Derived generator-input SHA-256: 1cb3fcc6fc2f89bec0e6552884dd6f29f4adc154f1b49ed4ef3a8cac11c108b4
- Generator input: -i /output/openapi-generator-input.yaml, derived from the exact canonical pin
- Generator-input adapter: mobile/android/scripts/api-generator-input.fish
- Single transformed schema: #/components/schemas/PortableImportUnprocessableResponse
- Transformation: closed oneOf becomes the previous flat optional typed data/error fields for Kotlin okhttp4/Moshi generation only
- Validation limitation: Android retains its previous permissive decoder; the canonical server and Swift contract remain strict. The derived input is not validation-equivalent or byte-identical to the canonical source.
- Generator image: openapitools/openapi-generator-cli:v7.20.0@sha256:fa4add01856e44becf70674164df354d61bd37ba0f444d27be949801e013921b
- Generator: OpenAPI Generator 7.20.0
- Release generator configuration: mobile/android/openapi-generator-config.yaml plus openapi-generator-release.ignore
- Release-safe generated Kotlin artifacts: mobile/android/phone/src/main/kotlin/io/medtracker/client
