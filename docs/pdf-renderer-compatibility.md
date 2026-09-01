# PDF renderer compatibility record

This record covers the foundation for issue #1997. Report endpoints remain on
Prawn until their later migration tasks.

## Dependency and architecture evidence

`sghtmltopdf` is locked at `0.1.1`. The lockfile retains `prawn` and
`prawn-table` and includes the published native gems
`sghtmltopdf-0.1.1-aarch64-linux` and
`sghtmltopdf-0.1.1-x86_64-linux`.

The rebuilt test image loads both renderers: `prawn=2.5.0` and
`sghtmltopdf=0.1.1`.

The ARM64 application image was built natively on 1 September 2026 with:

```text
docker buildx build --platform linux/arm64 --target app \
  --build-arg APP_IMAGE_REF=med-tracker:local-production-build \
  --tag med-tracker:issue-1997-arm64 --load .
```

It completed successfully, selected `sghtmltopdf 0.1.1 (aarch64-linux)`, and
produced an image of 295,072,076 bytes (281.4 MiB). The runtime image has no
`rustc` or `clang`; the native gem removes the need for Rust or libclang while
resolving MedTracker's dependencies.

The same `linux/amd64` build was attempted through Docker Desktop emulation on
this ARM64 host. It reached the production dependency-install stage but could
not complete before the local command runner's execution boundary terminated
the emulated build. The earlier disposable Ruby 4.0.6 proof loaded the
published `sghtmltopdf-0.1.1-x86_64-linux` gem and rendered with a TTF, but it
is not a substitute for a completed MedTracker application-image check. Run
the command above with `--platform linux/amd64` in CI or an AMD64-capable
builder before the lower PR is accepted.

No clean pre-change application image was available in this worktree, so an
overall image-size delta was not measured. The committed font and licence add
573,585 bytes to the source tree: 569,208 bytes for the TTF and 4,377 bytes for
the licence.

## Renderer and font evidence

`vendor/fonts/NotoSans-Regular.ttf` is the upstream static Noto Sans TTF.
`vendor/fonts/OFL-1.1.txt` is the matching upstream SIL Open Font Licence 1.1,
including the Noto Project copyright notice. The TTF has the Latin glyphs used
by English, Welsh, Spanish, Irish and Portuguese; the contract renders
representative characters from all five locales.

The application test image passed this contract:

```text
task test TEST_FILE=spec/services/reports/sghtmltopdf_contract_spec.rb
1 example, 0 failures
```

It configured the bundled TTF, returned `%PDF-1.7`, embedded the Noto Sans
subset, and exposed the accented locale glyphs through the PDF ToUnicode map.
A production ARM64 container repeated the in-process render after one warm-up.
Ten small renders averaged 0.69 ms each and the process reported a 24,348 kB
`VmHWM`. This is a representative renderer-only measurement, not an end-to-end
report benchmark.

WOFF and WOFF2 are not renderer inputs. `sghtmltopdf` supports TTF and OTF;
the static TTF is therefore deliberately bundled instead of converting or
reusing web-font assets.

## Operating constraints

Rendering stays in-process. The renderer releases Ruby's GVL while converting,
and server mode is intended for unsupported platforms or deliberately moving
CPU work to another process. MedTracker supports the required native gems, and
server mode would add a service, network hop and separately managed font and
asset policy without a present need.

The renderer does not execute JavaScript. Avoid unsupported or materially
limited CSS including multi-column layout, gradients, animations, filters,
`position: sticky`, `inline-flex`, `inline-grid`, subgrid, vertical writing,
right-to-left direction handling and the `font` shorthand. Use the supported
longhand properties and print layout rules in the shared renderer work.

The renderer does not produce tagged PDF, PDF/UA or other accessibility
conformance. It also does not support PDF/A, PDF/X, bookmarks, AcroForms,
encryption or electronic signatures. PDFs remain print/share artefacts; the
HTML report screens remain the accessible alternative.
