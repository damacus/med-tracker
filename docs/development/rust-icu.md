# ICU for the root Rust application

The published Better Auth 1.0.0-alpha.5 release requires the ICU 78 ABI to
build its unconditional ICU dependency. Its in-memory sorting path can also
need ICU at runtime. Root Rust Task commands set
`RUST_ICU_MAJOR_VERSION_NUMBER=78` and discover `RUST_ICU_LINK_SEARCH_DIR`
through the native package installation. Run them directly, for example
`task ci`; do not set these variables by hand. Documentation, frontend,
native client generation and the separate client-tools workspace do not
need ICU.

On macOS, install the versioned Homebrew formula with
`brew install icu4c@78`. Rust tasks use `brew --prefix icu4c@78` to find its
library directory. On Linux, install ICU 78 development libraries and
`pkg-config`. The four root Rust CI jobs use Ubuntu 26.04 and install
`fish pkg-config libicu-dev` through APT. Rust tasks discover the native
library directory with `pkg-config --variable=libdir icu-uc`. The system
loader finds these native libraries without a custom `LD_LIBRARY_PATH`.
ICU 78.2 and 78.3 share the required major ABI; the dependency uses its
pre-generated ICU 78 bindings.

Release builds also need the ICU 78 development libraries, and application
paths using ICU need the corresponding runtime libraries. A root Rust
deployment image and runtime installation are outside the current delivery
scope. CI verifies the actual Rust unit, integration and browser suites.
