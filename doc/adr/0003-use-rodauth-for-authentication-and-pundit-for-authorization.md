# 3. Use Rodauth for Authentication and Pundit for Authorization

Date: 2026-04-13

## Status

Accepted

## Context

Need robust, security-first authentication and flexible, testable authorization for multiple user roles (admin, doctor, nurse, carer, parent).

## Decision

- **Authentication:** Use `Rodauth` (`rodauth-rails`) for its security-focused, PostgreSQL-optimized design and built-in 2FA/OAuth support.
- **Authorization:** Use `Pundit` for explicit, PORO-based authorization with a deny-by-default strategy.

## Consequences

- Rodauth's Roda-based DSL has a learning curve.
- Pundit's explicit policies are easy to unit-test and understand.
- Strong security foundation for health-sector compliance.
