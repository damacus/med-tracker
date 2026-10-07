# Local password blocklist

`common-passwords.txt` is the NCSC 100,000 most used passwords corpus distributed by SecLists. The pinned source contains 99,840 lines. Passwords remain on this server; validation does not query a remote service.

- Source: https://github.com/danielmiessler/SecLists/blob/49c3b2d1d2481572bd7b0cb5af875a73cdf9d08e/Passwords/Common-Credentials/100k-most-used-passwords-NCSC.txt
- Version: SecLists commit `49c3b2d1d2481572bd7b0cb5af875a73cdf9d08e`, retrieved 7 October 2026.
- SHA-256: `c2e5696882c603b76bb67a47ee970897e5a76fc4c3f5547abe3d0ca340c576e0`.
- Licence: MIT, retained in `LICENSE`.

The application compares the complete proposed password, case-insensitively, against this local corpus when creating or changing a password. Existing password verification is unaffected. This is a common breached-password corpus, not an exhaustive list of every compromised password.

Review the upstream corpus quarterly and with security dependency updates. Download the file from a reviewed immutable SecLists commit, retain the licence, verify its checksum and line count, and replace the corpus and provenance together in a reviewed pull request. Run the normal password signup and password-change acceptance cases and the full auth quality gates before release. Runtime instances use the embedded version until a new build is released.
