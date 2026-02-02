# frozen_string_literal: true

module TwoFactorHelpers
  # Clear all 2FA methods for an account to allow direct login
  # This is a shared helper to avoid duplication across test files
  def clear_2fa_for_account(account)
    return unless account

    AccountOtpKey.where(id: account.id).delete_all
    AccountRecoveryCode.where(id: account.id).delete_all

    # Use safe navigation to handle cases where association might not be loaded
    account.account_webauthn_keys.destroy_all if account.respond_to?(:account_webauthn_keys)
  end
end

RSpec.configure do |config|
  config.include TwoFactorHelpers
end
