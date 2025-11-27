# ADR 0002: Authentication and Authorization Strategy

- Status: Accepted
- Date: 2025-11-27

## Context

MedTracker requires robust authentication and authorization to protect sensitive medication data. The application serves multiple user roles (administrators, doctors, nurses, carers, parents) with different access levels. UK healthcare compliance (GDPR, DTAC) mandates strong identity verification and audit trails.

Key requirements:

- Role-based access control for 6 user roles
- Support for email/password and OAuth (Google) authentication
- Email verification for account security
- Audit logging of authentication events
- Future support for two-factor authentication (2FA)

## Decision

### Authentication: Rodauth

We adopt **Rodauth** (`rodauth-rails`) as the primary authentication framework, replacing the initial `has_secure_password` implementation.

**Rationale:**

1. **Feature-complete**: Built-in support for email verification, password reset, remember me, OAuth, and 2FA
2. **Security-focused**: Designed with security as the primary concern, not bolted on
3. **PostgreSQL-optimized**: Works excellently with our PostgreSQL-only strategy
4. **Extensible**: Easy to customize flows without monkey-patching
5. **Maintained**: Active development with strong security track record

**Implementation:**

- `Account` model for authentication (separate from `Person` for demographics)
- `rodauth-omniauth` for Google OAuth integration
- Environment-specific email verification (strict in production, 7-day grace period in development)
- Legacy `User` model retained during transition period

### Authorization: Pundit

We adopt **Pundit** for authorization with deny-by-default policies.

**Rationale:**

1. **Simple and explicit**: Plain Ruby objects, easy to test and understand
2. **Rails conventions**: Follows Rails patterns, integrates cleanly
3. **Flexible**: Supports complex authorization logic without framework constraints
4. **Testable**: Policies are easily unit-tested with RSpec

**Implementation:**

- Policy classes for each resource (`UserPolicy`, `PersonPolicy`, `PrescriptionPolicy`, etc.)
- `ApplicationPolicy` with deny-by-default approach
- `policy_scope` for data filtering based on user role
- Comprehensive policy tests using `pundit-matchers`

### Role Hierarchy

| Role          | Access Level                               |
|---------------|--------------------------------------------|
| Administrator | Full system access, user management        |
| Doctor        | All patients, prescriptions, clinical data |
| Nurse         | All patients, medication recording         |
| Carer         | Assigned patients only                     |
| Parent        | Own children only                          |
| Minor         | Own data only (limited)                    |

### Audit Trail: PaperTrail

We use **PaperTrail** for audit logging of all authentication and data changes.

**Rationale:**

1. **Battle-tested**: Widely used in production healthcare applications
2. **Compliance**: Meets UK healthcare audit requirements
3. **Integration**: Works seamlessly with Pundit and Rodauth

## Consequences

### Positive

- Strong security foundation for healthcare compliance
- Clear separation of authentication (Rodauth) and authorization (Pundit)
- Comprehensive audit trail for regulatory requirements
- Testable, maintainable authorization logic
- Future-ready for 2FA and advanced security features

### Negative

- Dual authentication systems during migration (legacy + Rodauth)
- Learning curve for Rodauth's Roda-based DSL
- Additional complexity in Account/Person/User relationships

### Migration Path

1. **Phase 1** (Complete): Pundit authorization framework
2. **Phase 2** (In Progress): Rodauth foundation installed, login working
3. **Phase 3** (Pending): Rodauth signup with Person creation
4. **Phase 4** (Pending): Google OAuth integration
5. **Phase 5** (Pending): Legacy auth removal, user migration

## Related Documents

- `docs/plans/USER_MANAGEMENT_PLAN.md` - Overall user management strategy
- `docs/plans/USER_SIGNUP_PLAN.md` - Rodauth signup implementation status
- `docs/plans/RODAUTH_SIGNUP_IMPLEMENTATION.md` - Detailed implementation plan
- `docs/plans/USER_SIGNUP_AND_2FA_PLAN.md` - 2FA implementation plan
- `docs/plans/AUTHORIZATION_COMPLETION_PLAN.md` - Pundit implementation details
