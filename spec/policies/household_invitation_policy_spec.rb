# frozen_string_literal: true

require 'rails_helper'
require 'pundit/rspec'

RSpec.describe HouseholdInvitationPolicy, type: :policy do
  subject(:policy) { described_class.new(current_user, invitation) }

  let(:invitation) { HouseholdInvitation.new(email: 'test@example.com', membership_role: :member) }

  context 'with a household manager context' do
    let(:current_user) { household_policy_member(role: :owner).fetch(:context) }

    it 'permits invitation administration', :aggregate_failures do
      expect(policy.index?).to be true
      expect(policy.create?).to be true
      expect(policy.resend?).to be true
      expect(policy.destroy?).to be true
    end
  end

  context 'with a household member context' do
    let(:current_user) { household_policy_member(role: :member).fetch(:context) }

    it 'forbids invitation administration', :aggregate_failures do
      expect(policy.index?).to be false
      expect(policy.create?).to be false
      expect(policy.resend?).to be false
      expect(policy.destroy?).to be false
    end
  end

  context 'with a non-household account context' do
    let(:current_user) { User.new(email_address: 'legacy-admin@example.test') }

    it 'forbids invitation administration', :aggregate_failures do
      expect(policy.index?).to be false
      expect(policy.create?).to be false
      expect(policy.resend?).to be false
      expect(policy.destroy?).to be false
    end
  end
end
