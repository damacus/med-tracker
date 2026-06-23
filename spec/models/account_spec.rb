# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Account do
  fixtures :accounts

  describe 'associations' do
    it { is_expected.to have_one(:person).dependent(:nullify) }
  end

  describe 'enum' do
    it { is_expected.to define_enum_for(:status).with_values(unverified: 1, verified: 2, closed: 3) }

    it 'does not expose subscription plans as an account authorization attribute' do
      expect(described_class.defined_enums).not_to have_key('subscription_plan')
    end
  end

  describe 'versioning' do
    it 'creates a version when account status changes' do
      account = accounts(:jane_doe)

      expect do
        account.update!(status: :closed)
      end.to change { PaperTrail::Version.where(item_type: 'Account', item_id: account.id).count }.by(1)

      expect(PaperTrail::Version.where(item_type: 'Account', item_id: account.id).last.event).to eq('update')
    end

    it 'does not store password hashes in account versions' do
      account = accounts(:jane_doe)

      account.update!(email: 'audited-jane@example.com')

      version = PaperTrail::Version.where(item_type: 'Account', item_id: account.id).last
      expect(version.object.to_s).not_to include('password_hash')
    end
  end
end
