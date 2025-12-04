---
trigger: model_decision
description: When writing or modifying Ruby code in app/, lib/, or spec/
---

# Code Style and Architecture

## General Principles

- **Style Guide**: Adhere to the community Ruby Style Guide, enforced by RuboCop
- **Self-Documenting Code**: Write clear, self-documenting code. Use descriptive names and extract complex logic into well-named private methods
- **Functional-Style Ruby**: Prefer Enumerable methods (`map`, `select`, `reduce`) over imperative loops
- **Guard Clauses**: Use early returns to reduce nesting

## Architecture Patterns

- **Service Objects**: For complex business logic that doesn't fit in a model or controller, use POROs or Service Objects
- **Thin Controllers**: Controllers should only handle HTTP concerns; delegate business logic to models or services
- **Phlex Components**: Use Phlex for view components in `app/components/`

## Comment Accuracy

- If code behavior changes, update or remove related comments immediately
- Comments must match implementation - misleading comments are worse than no comments
- Prefer self-documenting code over comments
