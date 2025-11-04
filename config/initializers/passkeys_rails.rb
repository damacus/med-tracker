# frozen_string_literal: true

require 'passkeys-rails'

PasskeysRails.config do |c|
  c.default_class = 'User'
  c.class_whitelist = %w[User]

  c.wa_origin = ENV.fetch('PASSKEYS_ORIGIN') { "http://#{ENV.fetch('DEFAULT_HOST', 'localhost:3000')}" }
  c.wa_relying_party_name = 'MedTracker'
  c.wa_credential_options_timeout = 120_000
end
