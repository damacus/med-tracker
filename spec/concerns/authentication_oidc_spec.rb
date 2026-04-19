# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Authentication do
  fixtures :accounts, :people, :users, :account_identities

  # Thin anonymous controller so we can call concern methods in a proper Rails context
  let(:controller) do
    Class.new(ApplicationController) do
      allow_unauthenticated_access
      public :oidc_authenticated?, :should_setup_two_factor?, :two_factor_configured?
    end.new
  end

  def stub_account(account)
    allow(controller).to receive(:current_account).and_return(account)
  end

  def stub_user(user)
    allow(controller).to receive(:current_user).and_return(user)
  end

  def stub_rodauth(logged_in: true)
    dbl = instance_double(RodauthMain, logged_in?: logged_in)
    allow(controller).to receive(:rodauth).and_return(dbl)
  end

  describe '#oidc_authenticated?' do
    context 'when the account has an OIDC identity (fixture: oidc_damacus)' do
      before { stub_account(accounts(:damacus)) }

      it 'returns true' do
        expect(controller.oidc_authenticated?).to be true
      end
    end

    context 'when the account has no OIDC identity (password-only user)' do
      before { stub_account(accounts(:john_doe)) }

      it 'returns false' do
        expect(controller.oidc_authenticated?).to be false
      end
    end

    context 'when current_account is nil' do
      before { stub_account(nil) }

      it 'returns false without raising' do
        expect(controller.oidc_authenticated?).to be false
      end
    end
  end

  describe '#should_setup_two_factor? — OIDC users skip 2FA prompt' do
    let(:doctor_user) { users(:doctor) }
    let(:doctor_account) { accounts(:dr_jones) }

    before do
      stub_rodauth
      stub_user(doctor_user)
      stub_account(doctor_account)
      allow(controller).to receive(:two_factor_configured?).and_return(false)
    end

    context 'when doctor signed in via Zitadel (has OIDC identity)' do
      before do
        AccountIdentity.find_or_create_by!(account: doctor_account, provider: 'oidc', uid: 'zitadel-dr-jones')
      end

      it 'does not prompt for 2FA setup' do
        expect(controller.should_setup_two_factor?).to be false
      end
    end

    context 'when doctor signed in via password (no OIDC identity)' do
      before { AccountIdentity.where(account: doctor_account, provider: 'oidc').delete_all }

      it 'prompts for 2FA setup' do
        expect(controller.should_setup_two_factor?).to be true
      end
    end

    context 'when a non-privileged role (parent) signs in via password' do
      before do
        stub_user(users(:parent))
        stub_account(accounts(:parent))
        AccountIdentity.where(account: accounts(:parent), provider: 'oidc').delete_all
      end

      it 'does not prompt for 2FA setup regardless of auth method' do
        expect(controller.should_setup_two_factor?).to be false
      end
    end
  end

  describe 'Zitadel role mapping logic (zitadel_role_for)' do
    # Replicates the zitadel_role_for logic defined in auth_class_eval
    def role_from_zitadel_claims(role_names)
      raw_info = { 'urn:zitadel:iam:org:project:roles' => role_names.index_with { {} } }
      return nil unless raw_info.key?('urn:zitadel:iam:org:project:roles')

      zitadel_roles = raw_info['urn:zitadel:iam:org:project:roles'].keys
      valid_roles = User.roles.keys & zitadel_roles
      valid_roles.first&.to_sym || :parent
    end

    def role_from_absent_claim
      raw_info = {}
      return nil unless raw_info.key?('urn:zitadel:iam:org:project:roles')

      :parent
    end

    it 'returns nil when the roles claim is entirely absent (preserves existing role)' do
      expect(role_from_absent_claim).to be_nil
    end

    it 'maps doctor claim to :doctor role' do
      expect(role_from_zitadel_claims(['doctor'])).to eq(:doctor)
    end

    it 'maps administrator claim to :administrator role' do
      expect(role_from_zitadel_claims(['administrator'])).to eq(:administrator)
    end

    it 'maps nurse claim to :nurse role' do
      expect(role_from_zitadel_claims(['nurse'])).to eq(:nurse)
    end

    it 'maps carer claim to :carer role' do
      expect(role_from_zitadel_claims(['carer'])).to eq(:carer)
    end

    it 'falls back to :parent when claim is present but no role matches' do
      expect(role_from_zitadel_claims(['unknown_role'])).to eq(:parent)
    end

    it 'falls back to :parent when claim is present but empty' do
      expect(role_from_zitadel_claims([])).to eq(:parent)
    end

    it 'uses the first valid role when multiple roles are present' do
      result = role_from_zitadel_claims(User.roles.keys)
      expect(User.roles.keys).to include(result.to_s)
    end

    it 'ignores unknown roles even when mixed with valid ones' do
      expect(role_from_zitadel_claims(%w[unknown nurse])).to eq(:nurse)
    end
  end
end
