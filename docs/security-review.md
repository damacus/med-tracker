# Security Review - 2025-12-02

## Summary

- **Critical**: 0 issues
- **High**: 1 issue (timing restriction bypass)
- **Medium**: 0 issues
- **Low**: 0 issues
- **Informational**: 2 notes

## Static Analysis Results

### Brakeman

- **Warnings**: 0
- **Ignored**: 1 (false positive - search params in Admin::UsersController)

### Bundle Audit

- **Vulnerabilities**: 0

## Findings

### [HIGH] - Medication Timing Restrictions Not Enforced Server-Side

- **Location**: `app/controllers/prescriptions_controller.rb:116-126`, `app/controllers/person_medicines_controller.rb:65-72`
- **Type**: Business Logic Bypass
- **Description**: The `take_medicine` actions in both controllers create `MedicationTake` records without validating `can_take_now?`. Timing restrictions (`max_daily_doses`, `min_hours_between_doses`) are only enforced in the UI via disabled buttons, not server-side.
- **Risk**: A malicious user could bypass timing restrictions by directly calling the API, potentially leading to medication overdose. This is critical for a healthcare application.
- **Remediation**: Add server-side validation in both controllers to check `can_take_now?` before creating medication takes. Return an error if timing restrictions would be violated.
- **Status**: Fixed (see commit)

## Verified Security Controls

### Authentication

- [x] Passwords hashed with bcrypt (Rodauth)
- [x] Session tokens regenerated on login (Rodauth active_sessions)
- [x] Session expiry configured (30 min inactivity, 24 hour max)
- [x] Failed login rate limiting (Rack::Attack - 5 attempts/20 seconds)
- [x] Account lockout after 5 failed attempts (30 minute lockout)
- [x] Secure cookie flags (Rodauth defaults)
- [x] 2FA required for admin/doctor/nurse roles

### Authorization

- [x] All controllers have authorization checks (Pundit)
- [x] Deny-by-default ApplicationPolicy
- [x] Role-based access properly enforced
- [x] Admin actions restricted to administrators
- [x] Data scoping via policy_scope

### Input Validation

- [x] All controller actions use strong parameters (params.expect)
- [x] No mass assignment vulnerabilities
- [x] Model validations present

### Database Security

- [x] No raw SQL or string interpolation in queries
- [x] Parameterized queries used throughout

### Sensitive Data Handling

- [x] No hardcoded secrets in source code
- [x] Credentials properly encrypted (Rails credentials)
- [x] Sensitive files gitignored (env files, master.key, credential keys)
- [x] Password fields excluded from audit logs (ignore: password_digest)

### HTTP Security Headers

- [x] Content Security Policy configured
- [x] X-Frame-Options via CSP frame-ancestors
- [x] Permissions-Policy header set
- [x] Force SSL in production
- [x] Strict-Transport-Security enabled

### Rate Limiting

- [x] General request throttling (300/5 min per IP)
- [x] Login throttling (5/20 sec per IP and email)
- [x] Account creation throttling (3/min per IP)
- [x] Password reset throttling (5/min per IP, 5/hour per email)

### Audit Trail

- [x] PaperTrail enabled on critical models (User, Person, CarerRelationship, MedicationTake)
- [x] Audit logs include IP address
- [x] Audit logs include user who made change
- [x] Admin-only access to audit logs

### Healthcare-Specific

- [x] Medication changes logged (PaperTrail on MedicationTake)
- [x] Carer relationships properly enforced
- [x] Role-based data scoping (carers see only assigned patients)
- [x] Timing restrictions enforced server-side (after fix)

## Informational Notes

### Unused HomeController

- **Location**: `app/controllers/home_controller.rb`
- **Note**: Controller exists but is not routed. Consider removing if not needed.
- **Risk**: None (not accessible)

### Brakeman Ignored Warning

- **Location**: `app/controllers/admin/users_controller.rb:118`
- **Note**: Mass assignment warning for search params is a false positive. These params are used for filtering/sorting, not model updates.
- **Risk**: None (properly documented in brakeman.ignore)

## Recommendations

1. **Add model-level validation for timing restrictions** - Consider adding a custom validation to `MedicationTake` that checks timing restrictions on the source (prescription or person_medicine).

2. **Enable host authorization in production** - Uncomment and configure `config.hosts` in production.rb to prevent DNS rebinding attacks.

3. **Remove unused HomeController** - Clean up dead code.

## Next Review

Schedule next security review for: 2026-03-02 (quarterly)
