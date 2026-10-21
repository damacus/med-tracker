# ICU for the root Rust application

The pinned Better Auth revision links against ICU 78.3. Root Rust Task commands
find the ICU libraries and set `RUST_ICU_MAJOR_VERSION_NUMBER=78` and
`RUST_ICU_LINK_SEARCH_DIR` for Cargo. Run them directly, for example `task ci`;
do not set these variables by hand. Documentation, frontend, native client
generation and the separate client-tools workspace do not need ICU.

On macOS, install the versioned Homebrew formula with `brew install icu4c@78`.
The Task wrapper uses `brew --prefix icu4c@78` and checks that `icuuc`,
`icui18n` and `icudata` version 78 libraries exist. On supported Ubuntu x86_64
CI runners, `scripts/install-icu-linux.sh` downloads the official ICU 78.3
Ubuntu 22.04 archive, verifies its pinned SHA-256 and installs it under
`/opt/medtracker-icu`. The Task wrapper also sets `LD_LIBRARY_PATH` there so
test executables can load the same libraries they linked against. CI checks
their actual runtime linkage after running the Rust tests.

This setup applies to local development and CI. A root Rust deployment image
and runtime installation are outside the current delivery scope.
