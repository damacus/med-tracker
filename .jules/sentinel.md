## 2026-03-17 - Prevent Symbol Exhaustion
**Vulnerability:** Symbol exhaustion vulnerability caused by using `.to_sym` on user-provided parameter strings, even if validated.
**Learning:** Ruby symbols are not garbage collected in older versions, and even in newer versions (Ruby 2.2+), creating many dynamic symbols from user input is risky and can lead to DoS. In this codebase, `.to_sym` was used on validated `sort_direction` input.
**Prevention:** Always use a static Hash mapping (e.g., `{ 'asc' => :asc, 'desc' => :desc }[direction]`) when converting string parameters to symbols for query building or method calls.
## 2026-03-17 - Unvalidated Data Exposure via `to_unsafe_h`
**Vulnerability:** Unvalidated Data Exposure bypassing Strong Parameters by using `.to_unsafe_h` on ActionController::Parameters in view components to generate URLs.
**Learning:** Using `params.slice` followed by `.to_unsafe_h` only filters keys but does not ensure the values are scalar strings. An attacker could pass hashes or arrays via URL query parameters, potentially causing 500 server errors or subtle injection issues when these unvalidated values are rendered into pagination links or forms.
**Prevention:** Always use explicitly permitted parameters `params.permit(:key)` at the controller level before passing them to views or components, which allows the safe use of `.to_h` instead of `.to_unsafe_h`.
