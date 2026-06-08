# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Profiles' do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:user) { users(:damacus) }
  let(:account) { user.person.account }

  before do
    sign_in(user)
  end

  describe 'GET /profile' do
    it 'renders the profile page shell and key sections' do
      get profile_path

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('Profile settings')
      expect(response.body).to include('My Profile')
      expect(response.body).to include(user.name)
      expect(response.body).to include(account.email)
      expect(response.body).to include('Account Security')
      expect(response.body).to include('System Information')
      expect(response.body.scan('data-turbo-frame="modal"').size).to be >= 2
    end

    it 'renders the two-factor card and empty-state setup actions' do
      AccountOtpKey.where(id: account.id).delete_all
      AccountRecoveryCode.where(id: account.id).delete_all
      account.account_webauthn_keys.destroy_all

      get profile_path

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('Two-Factor Authentication')
      expect(response.body).to include('Authenticator App (TOTP)')
      expect(response.body).to include('Recovery Codes')
      expect(response.body).to include('Passkeys')
      expect(response.body).to include('Set up authenticator app')
      expect(response.body).to include('Generate recovery codes')
      expect(response.body).to include('No passkeys registered')
      expect(response.body).to include('Add a passkey')

      add_passkey_link = response.parsed_body.at_css('a[href="/webauthn-setup"]')
      expect(add_passkey_link['class']).to include('border-outline')
    end

    it 'renders configured two-factor states without a browser round-trip' do
      AccountOtpKey.find_or_create_by!(id: account.id) do |key|
        key.key = 'test_otp_key_secret'
      end
      AccountRecoveryCode.where(id: account.id).delete_all
      5.times do |i|
        code = "recovery-code-#{i}"
        recovery_code = AccountRecoveryCode.new(id: [account.id, code], code:)
        recovery_code.save!
      end
      account.account_webauthn_keys.destroy_all
      account.account_webauthn_keys.create!(
        webauthn_id: 'test-id',
        public_key: 'test-key',
        sign_count: 0,
        nickname: 'Test Passkey'
      )

      get profile_path

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.at_css('svg.material-symbol-passkey')).to be_present
      expect(response.body).to include('Authenticator app is active')
      expect(response.body).to include('Disable')
      expect(response.body).to include('Recovery codes generated')
      expect(response.body).to include('View codes')
      expect(response.body).to include('Regenerate')
      expect(response.body).to include('Test Passkey')
      expect(response.body).to include('Remove')
    end
  end

  describe 'PATCH /profile' do
    it 'uploads the signed-in person avatar' do
      file = Tempfile.new(['avatar', '.png'])
      file.write('avatar')
      file.rewind

      patch profile_path,
            params: { person: { avatar: Rack::Test::UploadedFile.new(file.path, 'image/png') } }

      expect(response).to redirect_to(profile_path)
      expect(user.person.reload.avatar).to be_attached, flash[:alert]
    ensure
      file&.close
      file&.unlink
    end

    it 'rejects invalid avatar uploads without attaching them' do
      file = Tempfile.new(['avatar', '.txt'])
      file.write('not an image')
      file.rewind

      patch profile_path,
            params: { person: { avatar: Rack::Test::UploadedFile.new(file.path, 'text/plain') } }

      expect(response).to redirect_to(profile_path)
      expect(flash[:alert]).to include('must be a PNG, JPEG, or WebP image')
      expect(user.person.reload.avatar).not_to be_attached
    ensure
      file&.close
      file&.unlink
    end

    it 'keeps the existing avatar when an invalid replacement is uploaded' do
      user.person.avatar.attach(io: StringIO.new('avatar'), filename: 'avatar.png', content_type: 'image/png')
      original_blob = user.person.avatar.blob
      file = Tempfile.new(['avatar', '.txt'])
      file.write('not an image')
      file.rewind

      patch profile_path,
            params: { person: { avatar: Rack::Test::UploadedFile.new(file.path, 'text/plain') } }

      expect(response).to redirect_to(profile_path)
      expect(flash[:alert]).to include('must be a PNG, JPEG, or WebP image')
      expect(user.person.reload.avatar).to be_attached
      expect(user.person.avatar.blob).to eq(original_blob)
    ensure
      file&.close
      file&.unlink
    end

    it 'updates the Gravatar opt-in preference' do
      patch profile_path, params: { user: { gravatar_enabled: '1' } }

      expect(response).to redirect_to(profile_path)
      expect(user.reload.gravatar_enabled?).to be(true)
    end

    it 'disables the Gravatar opt-in preference' do
      user.update!(gravatar_enabled: '1')

      patch profile_path, params: { user: { gravatar_enabled: '0' } }

      expect(response).to redirect_to(profile_path)
      expect(user.reload.gravatar_enabled?).to be(false)
    end

    it 'returns turbo_stream and updates flash when no changes are submitted' do
      patch profile_path, headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="flash"')
      expect(response.body).to include(I18n.t('profiles.no_changes'))
    end
  end

  describe 'DELETE /profile/avatar' do
    it 'removes the signed-in person avatar' do
      user.person.avatar.attach(io: StringIO.new('avatar'), filename: 'avatar.png', content_type: 'image/png')

      delete profile_avatar_path

      expect(response).to redirect_to(profile_path)
      expect(user.person.reload.avatar).not_to be_attached
    end

    it 'returns turbo_stream and refreshes the profile page shell' do
      user.person.avatar.attach(io: StringIO.new('avatar'), filename: 'avatar.png', content_type: 'image/png')

      delete profile_avatar_path, headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="main-content"')
      expect(response.body).to include(I18n.t('profiles.avatar.removed'))
    end
  end
end
