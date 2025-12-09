# Passkey/WebAuthn Setup Guide

MedTracker has the foundation to support passkey authentication via WebAuthn,
enabling passwordless login with biometrics or security keys. This guide covers
how to enable and configure WebAuthn for your deployment.

## Overview

Passkey/WebAuthn authentication can be enabled using [Rodauth](https://rodauth.jeremyevans.net/)
with the WebAuthn features. When implemented, the system will support:

- **Passwordless authentication** - Sign in with biometrics (Face ID, Touch ID, Windows Hello)
- **Security key support** - Use hardware security keys (YubiKey, etc.)
- **Phishing-resistant** - Cryptographically bound to your domain
- **Multi-device sync** - Passkeys sync via iCloud, Google Password Manager, etc.
- **Cross-device authentication** - Register on desktop, use from mobile

## What are Passkeys?

Passkeys are a modern, phishing-resistant replacement for passwords based on the
WebAuthn standard. They use public-key cryptography:

- **Private key** never leaves your device (stored in secure hardware)
- **Public key** stored on the server
- **Authentication** uses digital signatures, not shared secrets
- **Biometric authentication** (fingerprint, facial recognition) or device PIN

### Advantages over Passwords

| Feature | Passwords | Passkeys |
|---------|-----------|----------|
| Phishing-resistant | ❌ No | ✅ Yes |
| Unique per site | ❌ Often reused | ✅ Always unique |
| Secure storage | ⚠️ User's responsibility | ✅ OS-managed |
| Biometric auth | ❌ No | ✅ Yes |
| Works offline | ✅ Yes | ✅ Yes |
| Account recovery | ⚠️ Password reset | ✅ Device sync |

## Current Status

**Status**: Not currently implemented (deferred for future enhancement)

Passkeys were previously explored but the implementation was removed. The application
currently uses:
- Email/password authentication (Rodauth)
- OIDC/OAuth via Google
- Two-factor authentication (TOTP)

To enable passkeys, follow the implementation steps below.

## Implementation Steps

### 1. Add Required Dependencies

Add the WebAuthn gem to your `Gemfile`:

```ruby
# WebAuthn support for passkeys
gem 'webauthn'
```

Then run:

```bash
bundle install
```

### 2. Generate Database Migrations

Rodauth's WebAuthn features require two database tables:

```bash
rails generate migration CreateRodauthWebAuthnTables
```

Edit the generated migration:

```ruby
class CreateRodauthWebAuthnTables < ActiveRecord::Migration[8.0]
  def change
    create_table :account_webauthn_user_ids do |t|
      t.bigint :account_id, null: false
      t.string :webauthn_id, null: false
      t.timestamps
    end
    add_index :account_webauthn_user_ids, :account_id
    add_index :account_webauthn_user_ids, :webauthn_id, unique: true

    create_table :account_webauthn_keys do |t|
      t.bigint :account_id, null: false
      t.string :webauthn_id, null: false
      t.string :public_key, null: false
      t.integer :sign_count, null: false, default: 0
      t.datetime :last_use
      t.string :nickname
      t.timestamps
    end
    add_index :account_webauthn_keys, :account_id
    add_index :account_webauthn_keys, :webauthn_id
  end
end
```

Run the migration:

```bash
rails db:migrate
```

### 3. Enable WebAuthn in Rodauth

Edit `app/misc/rodauth_main.rb` and add the WebAuthn features:

```ruby
class RodauthMain < Rodauth::Rails::Auth
  configure do
    # Add WebAuthn features
    enable :webauthn_login, :webauthn_autofill
    
    # WebAuthn Relying Party (RP) configuration
    webauthn_rp_id { request.host }
    webauthn_rp_name 'MedTracker'
    webauthn_origin { "#{request.scheme}://#{request.host_with_port}" }
    
    # User verification requirement
    # Options: 'required', 'preferred', 'discouraged'
    webauthn_user_verification 'required'
    
    # Timeout for WebAuthn ceremony (milliseconds)
    webauthn_timeout 120_000 # 2 minutes
    
    # Credential types (always ['public-key'] for WebAuthn)
    webauthn_credential_types ['public-key']
    
    # Authenticator attachment
    # nil = allow both platform and cross-platform authenticators
    # 'platform' = only platform authenticators (Touch ID, Windows Hello)
    # 'cross-platform' = only security keys
    webauthn_authenticator_attachment nil
    
    # Resident key requirement (discoverable credentials)
    # 'required' = credential must be stored on authenticator
    # 'preferred' = store if possible, fallback to server-side
    # 'discouraged' = prefer server-side storage
    webauthn_resident_key 'preferred'
  end
end
```

### 4. Update Routes

Rodauth automatically creates WebAuthn routes:

- `/webauthn-setup` - Register a new passkey
- `/webauthn-auth` - Authenticate with a passkey
- `/webauthn-remove` - Remove a passkey

### 5. Update Login View

Add passkey login option to your login page:

```ruby
# In your login Phlex component
def passkey_section
  div(class: 'mt-6 border-t pt-6') do
    h3(class: 'text-sm font-medium mb-3') { 'Or sign in with a passkey' }
    
    form(action: webauthn_auth_path, method: 'post', data: { turbo: false }) do
      authenticity_token_field
      
      button(
        type: 'submit',
        class: 'w-full btn btn-outline'
      ) do
        # Icon
        svg(class: 'w-5 h-5 mr-2', viewBox: '0 0 24 24') do |s|
          s.path(d: 'M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z')
        end
        plain 'Sign in with passkey'
      end
    end
  end
end
```

### 6. Add Passkey Management UI

Create a settings page for managing passkeys:

```ruby
# app/views/settings/security.rb
module Views
  module Settings
    class Security < ApplicationView
      def template
        h2 { 'Passkeys' }
        
        if @passkeys.any?
          passkey_list
        else
          empty_state
        end
        
        add_passkey_button
      end
      
      private
      
      def passkey_list
        div(class: 'space-y-4') do
          @passkeys.each do |passkey|
            passkey_item(passkey)
          end
        end
      end
      
      def passkey_item(passkey)
        div(class: 'flex items-center justify-between p-4 border rounded') do
          div do
            p(class: 'font-medium') { passkey.nickname || 'Unnamed passkey' }
            p(class: 'text-sm text-gray-600') do
              plain "Added #{passkey.created_at.strftime('%B %d, %Y')}"
            end
            if passkey.last_use
              p(class: 'text-sm text-gray-600') do
                plain "Last used #{time_ago_in_words(passkey.last_use)} ago"
              end
            end
          end
          
          button_to(
            'Remove',
            webauthn_remove_path(id: passkey.id),
            method: :delete,
            class: 'btn btn-sm btn-outline-danger',
            data: { turbo_confirm: 'Remove this passkey?' }
          )
        end
      end
    end
  end
end
```

## Security Considerations

### Phishing Resistance

Passkeys are cryptographically bound to your domain:

- **Origin validation**: Credentials only work on the registered domain
- **Cannot be phished**: User cannot provide passkey to a fake site
- **No shared secrets**: Private key never leaves the authenticator

### User Verification

Configure user verification based on your security requirements:

```ruby
# Require biometric or PIN for all authentications
webauthn_user_verification 'required'

# Prefer user verification but allow presence-only
webauthn_user_verification 'preferred'

# Only check user presence (button press)
webauthn_user_verification 'discouraged'
```

### Attestation

Attestation proves the authenticator is genuine:

```ruby
# Request attestation from authenticator
webauthn_attestation 'direct'    # Full attestation statement
webauthn_attestation 'indirect'  # Anonymized attestation
webauthn_attestation 'none'      # No attestation (default)
```

For healthcare applications, consider `'direct'` attestation to verify
authenticator security properties.

### Counter Validation

WebAuthn includes a signature counter to detect cloned credentials:

```ruby
# In your after_webauthn_authentication hook
after_webauthn_authentication do
  # Check if counter decreased (potential clone)
  if webauthn_key[:sign_count] > new_sign_count
    # Alert user and potentially revoke credential
    Rails.logger.warn("Potential cloned credential detected for account #{account_id}")
    # Send security alert email
  end
end
```

## Browser Support

Passkey support varies by browser and operating system:

| Browser | Platform | Status |
|---------|----------|--------|
| Chrome | All | ✅ Full support |
| Safari | macOS/iOS | ✅ Full support |
| Firefox | All | ✅ Full support |
| Edge | All | ✅ Full support |

All modern browsers support WebAuthn Level 2+ with passkey sync.

## Platform Authenticators

### Apple

- **Touch ID** (Mac, iPhone, iPad)
- **Face ID** (iPhone, iPad)
- **iCloud Keychain sync** across Apple devices

### Google

- **Fingerprint** (Android phones)
- **Face unlock** (Android phones)
- **Google Password Manager** syncs across Chrome/Android

### Microsoft

- **Windows Hello** (Face recognition, fingerprint, PIN)
- **Microsoft Authenticator** for cross-device

## Testing

### Test Credentials

For testing, browsers provide virtual authenticators:

```javascript
// In Chrome DevTools
// Settings > More tools > WebAuthn
// Create a virtual authenticator
```

### Test User Flows

1. **Registration**: Create account → Add passkey → Verify stored
2. **Login**: Use passkey → Verify authenticated
3. **Management**: Add multiple passkeys → Remove one → Verify still works
4. **Cross-device**: Register on desktop → Use QR code → Authenticate on mobile

## Troubleshooting

### Common Issues

#### "Passkey not available"

**Cause**: Browser or OS doesn't support WebAuthn
**Solution**: Check browser compatibility, ensure HTTPS (required for WebAuthn)

#### "RP ID mismatch"

**Cause**: Relying Party ID doesn't match domain
**Solution**: Ensure `webauthn_rp_id` matches your domain

```ruby
# Correct: For https://app.medtracker.com
webauthn_rp_id { 'medtracker.com' }

# Also valid
webauthn_rp_id { 'app.medtracker.com' }
```

#### "Origin not allowed"

**Cause**: Origin doesn't match registered RP ID
**Solution**: Verify origin configuration includes protocol and port

```ruby
webauthn_origin { "#{request.scheme}://#{request.host_with_port}" }
```

### Debug Mode

Enable WebAuthn debug logging:

```ruby
# In development environment
if Rails.env.development?
  WebAuthn.configure do |config|
    config.logger = Rails.logger
    config.logger.level = Logger::DEBUG
  end
end
```

## Migration from Passwords

When migrating users from passwords to passkeys:

1. **Keep passwords**: Don't force users to remove passwords
2. **Gradual adoption**: Prompt users to add passkeys but don't require
3. **Recovery options**: Ensure users have alternative authentication methods
4. **Education**: Explain benefits of passkeys to users

## Related Documentation

- [Authentication Setup](authentication.md)
- [Security Overview](security.md)
- [Rodauth Documentation](https://rodauth.jeremyevans.net/rdoc/files/doc/webauthn_rdoc.html)
- [WebAuthn Specification](https://www.w3.org/TR/webauthn-2/)
- [Passkeys.dev Guide](https://passkeys.dev/)

## Future Enhancements

Planned improvements for passkey support:

- **Conditional UI**: Show passkeys in username field autofill
- **Cross-device QR codes**: Seamless mobile registration
- **Biometric hints**: Guide users to available authenticators
- **Backup authenticators**: Prompt users to register multiple passkeys
- **Security notifications**: Alert on suspicious authentication patterns
