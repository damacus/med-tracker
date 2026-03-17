## 2026-03-17 - Prevent Symbol Exhaustion
**Vulnerability:** Symbol exhaustion vulnerability caused by using `.to_sym` on user-provided parameter strings, even if validated.
**Learning:** Ruby symbols are not garbage collected in older versions, and even in newer versions (Ruby 2.2+), creating many dynamic symbols from user input is risky and can lead to DoS. In this codebase, `.to_sym` was used on validated `sort_direction` input.
**Prevention:** Always use a static Hash mapping (e.g., `{ 'asc' => :asc, 'desc' => :desc }[direction]`) when converting string parameters to symbols for query building or method calls.
