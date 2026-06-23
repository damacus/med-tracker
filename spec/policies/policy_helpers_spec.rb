# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PolicyHelpers, type: :policy do
  let(:host_class) do
    Class.new(ApplicationPolicy) do
      def active_membership_public? = active_membership?
      def household_owner_public? = household_owner?
      def household_administrator_public? = household_administrator?
      def household_manager_public? = household_manager?
      def admin_public? = admin?
    end
  end

  def policy_for(user) = host_class.new(user, :record)

  it 'detects active household owners as managers' do
    household = household_policy_member(role: :owner).fetch(:household)
    owner = household.household_memberships.owner.sole
    owner_context = AuthorizationContext.new(account: owner.account, household: household, membership: owner)

    expect(helper_results(owner_context, %i[
                            active_membership_public? household_owner_public? household_manager_public? admin_public?
                          ])).to eq(
                            active_membership_public?: true,
                            household_owner_public?: true,
                            household_manager_public?: true,
                            admin_public?: true
                          )
  end

  it 'detects active household administrators as managers' do
    household = household_policy_member(role: :owner).fetch(:household)
    administrator = household_policy_member(role: :administrator, household: household).fetch(:context)

    expect(helper_results(administrator, %i[
                            household_administrator_public? household_manager_public?
                          ])).to eq(
                            household_administrator_public?: true,
                            household_manager_public?: true
                          )
  end

  it 'does not treat plain household members or legacy users as managers' do
    member = household_policy_member(role: :member).fetch(:context)
    legacy_admin = User.new

    expect(policy_for(member).active_membership_public?).to be(true)
    expect(policy_for(member).household_manager_public?).to be(false)
    expect(policy_for(legacy_admin).active_membership_public?).to be(false)
    expect(policy_for(legacy_admin).admin_public?).to be(false)
    expect(policy_for(nil).admin_public?).to be(false)
  end

  def helper_results(user, helpers)
    helpers.index_with { |helper| policy_for(user).public_send(helper) }
  end
end
