# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Two-factor soft enforcement' do
  fixtures :accounts, :people, :users

  def clear_2fa_for(account)
    AccountOtpKey.where(id: account.id).delete_all
    account.account_webauthn_keys.destroy_all
  end

  describe 'GET /profile' do
    it 'shows the warning for administrators without 2FA' do
      user = users(:damacus)
      clear_2fa_for(user.person.account)
      sign_in(user)

      get profile_path

      expect(response.body).to include('For enhanced security, please set up two-factor authentication')
    end

    it 'shows the warning for doctors without 2FA' do
      user = users(:doctor)
      clear_2fa_for(user.person.account)
      sign_in(user)

      get profile_path

      expect(response.body).to include('For enhanced security, please set up two-factor authentication')
    end

    it 'shows the warning for nurses without 2FA' do
      user = users(:nurse)
      clear_2fa_for(user.person.account)
      sign_in(user)

      get profile_path

      expect(response.body).to include('For enhanced security, please set up two-factor authentication')
    end

    it 'does not show the warning for parents without 2FA' do
      user = users(:jane)
      clear_2fa_for(user.person.account)
      sign_in(user)

      get profile_path

      expect(response.body).not_to include('For enhanced security, please set up two-factor authentication')
    end

    it 'does not show the warning for carers without 2FA' do
      user = users(:bob)
      clear_2fa_for(user.person.account)
      sign_in(user)

      get profile_path

      expect(response.body).not_to include('For enhanced security, please set up two-factor authentication')
    end

    it 'does not show the warning when a privileged user has TOTP configured' do
      user = users(:damacus)
      sign_in(user)
      AccountOtpKey.find_or_create_by!(id: user.person.account.id) do |key|
        key.key = 'JBSWY3DPEHPK3PXP'
      end

      get profile_path

      expect(response.body).not_to include('For enhanced security, please set up two-factor authentication')
    end

    it 'does not show the warning when a privileged user has a passkey configured' do
      user = users(:damacus)
      sign_in(user)
      user.person.account.account_webauthn_keys.create!(
        webauthn_id: 'test-id',
        public_key: 'test-key',
        sign_count: 0
      )

      get profile_path

      expect(response.body).not_to include('For enhanced security, please set up two-factor authentication')
    end
  end

  describe 'GET /dashboard' do
    it 'does not show the warning on the dashboard' do
      user = users(:damacus)
      clear_2fa_for(user.person.account)
      sign_in(user)

      get dashboard_path

      expect(response.body).not_to include('For enhanced security, please set up two-factor authentication')
    end
  end
end
