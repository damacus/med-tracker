---
trigger: always_on
---

# Code Style and Architecture

- **Style Guide**: Adhere to the community Ruby Style Guide, enforced by the standard RuboCop configuration.
- **Self-Documenting Code**: Write clear, self-documenting code. Avoid comments; use descriptive names and extract complex logic into well-named private methods.
- **Functional-Style Ruby**: Prefer Enumerable methods (`map`, `select`, `reduce`) over imperative loops.
- **Service Objects**: For complex business logic that doesn't fit in a model or controller, use Plain Old Ruby Objects (POROs) or Service Objects.
