# Read-only authorization-server feasibility

This bounded Scout question supports tranches 2 and 3 while foundation verification
continues. No edits, builds, installs, tests, Git/PR mutation, remote messages or
additional agents. Return findings to Bucky; Bucky records the evidence.

Inspect actual Rodauth/native/SMART configuration and existing Rust OAuth code.
Evaluate full maintained Rust authorization-server libraries, starting with the
existing oxide-auth dependency and async/HTTP adapters. Loco JWT and client OAuth
are not sufficient. Cover S256 PKCE, client authentication, expiring one-time code
grants, refresh rotation/revocation, SMART scopes/patient context and audited
household consent. Separate standard protocol enforcement from application
persistence/authorization adapter responsibilities.

Use Context7 and official source/docs. Cite exact interfaces, known limitations,
maintenance evidence and interoperability/negative acceptance tests. Do not infer
capability merely from a dependency name. Recommend a decision-ready in-process
library path only if evidence supports it. Identify any requirement that would
necessitate a materially different external authorization-server architecture;
do not silently add that service. No custom security-protocol shortcut.
