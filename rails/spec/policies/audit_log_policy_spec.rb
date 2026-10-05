# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AuditLogPolicy do
  subject(:policy) { described_class.new(user, :audit_log) }

  context 'when context has an owner membership' do
    let(:user) { context_with_membership(:owner) }

    it { is_expected.to permit_action(:index) }
    it { is_expected.to permit_action(:show) }
    it { is_expected.to forbid_action(:create) }
    it { is_expected.to forbid_action(:update) }
    it { is_expected.to forbid_action(:destroy) }
  end

  context 'when context has an administrator membership' do
    let(:user) { context_with_membership(:administrator) }

    it { is_expected.to permit_action(:index) }
    it { is_expected.to permit_action(:show) }
  end

  context 'when context has a member membership' do
    let(:user) { context_with_membership(:member) }

    it { is_expected.to forbid_action(:index) }
    it { is_expected.to forbid_action(:show) }
  end

  context 'when user is a legacy administrator' do
    let(:user) { User.new }

    it { is_expected.to forbid_action(:index) }
    it { is_expected.to forbid_action(:show) }
  end

  describe 'Scope' do
    it 'filters audit versions to the current household for managers' do
      context = context_with_membership(:owner)
      other_household = Household.create!(name: 'Other Audit Family', slug: "other-audit-#{SecureRandom.hex(4)}")
      matching = PaperTrail::Version.create!(
        event: 'update',
        item_type: 'Person',
        item_id: 1,
        household_id: context.household.id
      )
      PaperTrail::Version.create!(event: 'update', item_type: 'Person', item_id: 2, household_id: other_household.id)

      resolved = described_class::Scope.new(context, PaperTrail::Version.all).resolve

      expect(resolved).to contain_exactly(matching)
    end

    it 'returns no audit versions without household context' do
      context = context_with_membership(:owner)
      context = AuthorizationContext.new(account: context.account, household: nil, membership: context.membership)

      resolved = described_class::Scope.new(context, PaperTrail::Version.all).resolve

      expect(resolved).to be_empty
    end

    it 'returns no audit versions for non-managers' do
      context = context_with_membership(:member)

      resolved = described_class::Scope.new(context, PaperTrail::Version.all).resolve

      expect(resolved).to be_empty
    end
  end

  def context_with_membership(role)
    account = Account.create!(email: "audit-#{role}-#{SecureRandom.hex(4)}@example.test", status: :verified)
    household = Household.create!(name: "Audit #{role} Family", slug: "audit-#{role}-#{SecureRandom.hex(4)}")
    membership = household.household_memberships.create!(account: account, role: role, status: :active)

    AuthorizationContext.new(account: account, household: household, membership: membership)
  end
end
