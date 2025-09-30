---
description: Fix rubocop violations
auto_execution_mode: 1
---

- run `rubocop . -A`
- Fix all violations
- Decide which violations we need to ignore
- If we have existing files that we have ignored in .rubocop.yml make a plan to see if we can remove anything we are ignoring
