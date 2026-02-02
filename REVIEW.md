# Code Review - feature/passkey-webauthn-support

**Base Branch**: main
**Changed Files**: 50 Ruby files
**Review Date**: 2026-02-01

---

## Summary

This branch implements WebAuthn/Passkey support for MedTracker, adding passwordless authentication as a second-factor option alongside TOTP. The implementation integrates with Rodauth's built-in WebAuthn features and includes comprehensive UI components using Phlex.

**Key Changes:**

- New models: `AccountWebauthnKey`, `AccountWebauthnUserId`, `AccountOtpKey`, `AccountRecoveryCode`
- New Phlex views for 2FA setup/auth flows
- Profile page 2FA management card
- Soft enforcement of 2FA for privileged roles (admin, doctor, nurse)
- Database migrations for WebAuthn tables

---

## Critical Issues

### 1. Missing Foreign Key Constraint

[WebAuthn migration lacks foreign key constraint](file:///Users/damacus/repos/damacus/med-tracker/db/migrate/20260126161624_create_rodauth_web_authn_tables.rb#L6)

The `account_webauthn_keys` and `account_webauthn_user_ids` tables reference `account_id` but lack foreign key constraints:

```ruby
t.bigint :account_id, null: false  # No foreign key to accounts table
```

**Recommendation:** Add `add_foreign_key :account_webauthn_keys, :accounts` for referential integrity.

### 2. Potential N+1 Query in Recovery Codes Check

[Recovery codes count called multiple times](file:///Users/damacus/repos/damacus/med-tracker/app/views/profiles/two_factor_card.rb#L220)

```ruby
def recovery_codes_exist?
  recovery_codes_count.positive?  # Calls count query
end

def recovery_codes_count
  AccountRecoveryCode.where(id: account.id).count  # Another query
end
```

`recovery_codes_count` is called from both `recovery_codes_exist?` and `render_recovery_codes_actions`. Consider memoization.

---

## Design & Architecture

### OOP Violations

#### Sandi Metz Rule #1: Classes > 100 lines

| File                                                                                                              | Lines | Rating |
|-------------------------------------------------------------------------------------------------------------------|-------|--------|
| [two_factor_card.rb](file:///Users/damacus/repos/damacus/med-tracker/app/views/profiles/two_factor_card.rb#L1)    | 238   | ⚠️     |
| [otp_setup.rb](file:///Users/damacus/repos/damacus/med-tracker/app/views/rodauth/otp_setup.rb#L1)                 | 215   | ⚠️     |
| [two_factor_manage.rb](file:///Users/damacus/repos/damacus/med-tracker/app/views/rodauth/two_factor_manage.rb#L1) | 199   | ⚠️     |

**Note:** Phlex view components tend to be longer due to markup. Consider extracting reusable sub-components.

#### Method Complexity (TooManyStatements)

- [render_auth_method_card has 14 statements](file:///Users/damacus/repos/damacus/med-tracker/app/views/rodauth/two_factor_manage.rb#L116)
- [render_setup_form has 9 statements](file:///Users/damacus/repos/damacus/med-tracker/app/views/rodauth/webauthn_setup.rb#L92)
- [form_section has 8 statements](file:///Users/damacus/repos/damacus/med-tracker/app/views/rodauth/webauthn_auth.rb#L36)

#### Uncommunicative Variable Names

[Variable 's' used in SVG blocks](file:///Users/damacus/repos/damacus/med-tracker/app/views/rodauth/two_factor_manage.rb#L155)

```ruby
svg(...) do |s|
  s.path(...)
end
```

This is a common Phlex pattern for SVG builders, but consider renaming to `svg_builder` for clarity.

### Code Duplication (DRY Violations)

RubyCritic detected **significant duplication** across Rodauth views:

#### Pattern 1: Header Section (7 files)

Found in: `otp_auth.rb`, `otp_disable.rb`, `otp_setup.rb`, `two_factor_auth.rb`, `webauthn_auth.rb`, `webauthn_setup.rb`, `profiles/show.rb`

```ruby
def header_section
  div(class: 'mx-auto max-w-xl text-center space-y-3') do
    h1(class: 'text-3xl font-bold tracking-tight text-slate-800 sm:text-4xl') { ... }
    p(class: 'text-lg text-slate-600') { ... }
  end
end
```

**Recommendation:** Extract to shared `Views::Rodauth::HeaderSection` component.

#### Pattern 2: Decorative Glow (7 files)

[Identical decorative_glow method](file:///Users/damacus/repos/damacus/med-tracker/app/views/rodauth/otp_auth.rb#L30)

```ruby
def decorative_glow
  div(class: 'pointer-events-none absolute inset-x-0 top-24 flex justify-center opacity-60') do
    div(class: 'h-64 w-64 rounded-full bg-sky-200 blur-3xl sm:h-80 sm:w-80')
  end
end
```

**Recommendation:** Extract to `Views::Shared::DecorativeGlow` or a concern.

#### Pattern 3: Card Classes (7 files)

```ruby
def card_classes
  'w-full backdrop-blur bg-white/90 shadow-2xl border border-white/70 ring-1 ring-black/5 rounded-2xl'
end
```

**Recommendation:** Define as constant in `Views::Base` or extract to Tailwind component class.

#### Pattern 4: Form Section Structure (4 files)

[Duplicate form wrapper pattern](file:///Users/damacus/repos/damacus/med-tracker/app/views/rodauth/otp_auth.rb#L36)

The card + header + content structure is repeated identically.

### Rails Patterns

#### ✅ Good: Model Validations

[AccountWebauthnKey validations are comprehensive](file:///Users/damacus/repos/damacus/med-tracker/app/models/account_webauthn_key.rb#L6)

```ruby
validates :webauthn_id, presence: true, uniqueness: { scope: :account_id }
validates :public_key, presence: true
validates :sign_count, presence: true, numericality: { greater_than_or_equal_to: 0 }
validates :nickname, presence: true
```

#### ✅ Good: Proper Association Setup

[Account associations properly configured](file:///Users/damacus/repos/damacus/med-tracker/app/models/account.rb#L8)

```ruby
has_many :account_webauthn_keys, dependent: :destroy
has_many :account_webauthn_user_ids, dependent: :destroy
```

#### ⚠️ Consider: Table Existence Checks

[Repeated table_exists? checks](file:///Users/damacus/repos/damacus/med-tracker/app/views/profiles/two_factor_card.rb#L213)

```ruby
def totp_enabled?
  return false unless ActiveRecord::Base.connection.table_exists?('account_otp_keys')
  AccountOtpKey.exists?(id: account.id)
rescue StandardError
  false
end
```

These checks suggest the schema might not always be in a consistent state. Consider:
1. Running migrations before deployment
2. Using a feature flag instead of table existence checks

---

## Security Concerns

### ✅ Positive Security Patterns

1. **Password required for 2FA modifications**
   [two_factor_modifications_require_password? true](file:///Users/damacus/repos/damacus/med-tracker/app/misc/rodauth_main.rb#L127)

2. **Proper WebAuthn RP ID configuration**
   [Dynamic RP ID based on environment](file:///Users/damacus/repos/damacus/med-tracker/app/misc/rodauth_main.rb#L112)

3. **Session management configured**
   [30-minute inactivity, 24-hour absolute timeout](file:///Users/damacus/repos/damacus/med-tracker/app/misc/rodauth_main.rb#L102)

4. **Account lockout after failed attempts**
   [5 failed logins, 30-minute lockout](file:///Users/damacus/repos/damacus/med-tracker/app/misc/rodauth_main.rb#L95)

### ⚠️ Security Considerations

#### 1. WebAuthn Origin Configuration

[Origin uses ENV fallback to request.base_url](file:///Users/damacus/repos/damacus/med-tracker/app/misc/rodauth_main.rb#L120)

```ruby
webauthn_origin { ENV.fetch('APP_URL', request.base_url) }
```

Ensure `APP_URL` is always set in production to prevent origin mismatch attacks.

#### 2. Passkey Removal Without Confirmation

[Remove button triggers POST without re-authentication](file:///Users/damacus/repos/damacus/med-tracker/app/views/profiles/two_factor_card.rb#L157)

```ruby
button_to(
  'Remove',
  "/webauthn-remove?id=#{passkey.id}",
  method: :post,
  data: { turbo_confirm: 'Are you sure...' }
)
```

Consider requiring password re-entry for passkey removal (critical security action).

#### 3. CSRF Token Handling

[All forms properly include authenticity tokens](file:///Users/damacus/repos/damacus/med-tracker/app/views/rodauth/webauthn_setup.rb#L112) ✅

---

## Test Coverage

### ✅ Good Test Coverage

- [Passkey configuration tests](file:///Users/damacus/repos/damacus/med-tracker/spec/features/authentication/passkey_spec.rb#L1) - Database schema verification
- [Passkey registration tests](file:///Users/damacus/repos/damacus/med-tracker/spec/features/authentication/passkey_registration_spec.rb#L1) - UI flow testing
- [Two-factor management tests](file:///Users/damacus/repos/damacus/med-tracker/spec/features/profiles/two_factor_management_spec.rb#L1) - Comprehensive 266-line spec
- [Soft enforcement tests](file:///Users/damacus/repos/damacus/med-tracker/spec/system/two_factor_soft_enforcement_spec.rb#L1) - Role-based 2FA prompting

### ⚠️ Missing Test Scenarios

1. **WebAuthn authentication flow** - No end-to-end test for authenticating with a passkey (complex due to WebAuthn API mocking)
1. **Edge case: Multiple passkeys** - Tests only cover 0-1 passkeys
1. **Error handling** - Missing tests for WebAuthn failure scenarios
1. **Recovery code regeneration** - No test for the regenerate button functionality

---

## Tool Reports

### RubyCritic Summary

- **Overall Score**: 43.17 (out of 100)
- **Files with F Rating**: 8 files
- **Primary Issues**: Code duplication, method complexity
- **Code Smells**: 50+ detected

### Files Requiring Attention

| File                   | Rating | Main Issues                      |
|------------------------|--------|----------------------------------|
| `two_factor_manage.rb` | F      | Duplication, TooManyStatements   |
| `webauthn_auth.rb`     | F      | Duplication, IrresponsibleModule |
| `webauthn_setup.rb`    | F      | Duplication, TooManyStatements   |
| `otp_setup.rb`         | F      | Duplication, complexity          |
| `otp_auth.rb`          | F      | Duplication                      |
| `two_factor_card.rb`   | D      | ClassLength, complexity          |

### SimpleCov Summary

*Unable to run - Docker not available. Run `task test` to generate coverage report.*

---

## Recommendations

### High Priority

1. **Extract shared Rodauth view components**
   - Create `Views::Rodauth::Base` with common methods
   - Extract `HeaderSection`, `DecorativeGlow`, `CardWrapper` components
   - Define shared CSS class constants

2. **Add foreign key constraints** to WebAuthn tables in a follow-up migration

3. **Memoize database queries** in `TwoFactorCard`:

   ```ruby
   def recovery_codes_count
     @recovery_codes_count ||= AccountRecoveryCode.where(id: account.id).count
   end
   ```

### Medium Priority

1. **Improve variable naming** - Use `svg_builder` instead of `s` in SVG blocks

2. **Add WebAuthn error handling tests** once a WebAuthn mocking strategy is established

3. **Consider re-authentication for passkey removal** as an additional security measure

### Low Priority

1. **Remove table existence checks** if schema is guaranteed stable

2. **Add module documentation** to satisfy IrresponsibleModule warnings

---

## Positive Observations

1. **Clean Rodauth integration** - Uses Rodauth's built-in WebAuthn support rather than custom implementation
2. **Consistent UI patterns** - All 2FA views follow the same visual structure
3. **Good separation of concerns** - Models are thin, views handle presentation
4. **Comprehensive Phlex components** - RubyUI integration is well done
5. **Thoughtful security defaults** - Password required for 2FA changes, proper session timeouts
6. **Soft enforcement approach** - 2FA encouraged but not blocking for privileged users
7. **Good test coverage** - 266-line spec for 2FA management with multiple scenarios
8. **Proper migration structure** - Incremental migrations with proper defaults

---

## Files Changed

### Models (5)

- `app/models/account.rb`
- `app/models/account_otp_key.rb`
- `app/models/account_recovery_code.rb`
- `app/models/account_webauthn_key.rb`
- `app/models/account_webauthn_user_id.rb`

### Views (10)

- `app/views/profiles/show.rb`
- `app/views/profiles/two_factor_card.rb`
- `app/views/rodauth/otp_auth.rb`
- `app/views/rodauth/otp_disable.rb`
- `app/views/rodauth/otp_setup.rb`
- `app/views/rodauth/two_factor_auth.rb`
- `app/views/rodauth/two_factor_manage.rb`
- `app/views/rodauth/webauthn_auth.rb`
- `app/views/rodauth/webauthn_setup.rb`

### Components (4)

- `app/components/icons/key.rb`
- `app/components/icons/x_circle.rb`
- `app/components/layouts/flash.rb`
- `app/components/ruby_ui/alert/alert.rb`

### Controllers/Concerns (1)

- `app/controllers/concerns/authentication.rb`

### Configuration (2)

- `app/misc/rodauth_main.rb`
- `config/routes.rb`

### Migrations (4)

- `db/migrate/20251103230827_simplify_person_types.rb`
- `db/migrate/20260126161624_create_rodauth_web_authn_tables.rb`
- `db/migrate/20260126162420_add_unique_index_to_account_webauthn_keys.rb`
- `db/migrate/20260127134200_add_defaults_to_account_webauthn_keys_timestamps.rb`

### Specs (16)

- Various feature, system, and component specs
