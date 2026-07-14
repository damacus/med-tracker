# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PersonAccessGrantPolicy, type: :policy do
  let(:owner) { household_policy_member(role: :owner) }
  let(:household) { owner.fetch(:household) }
  let(:administrator) { household_policy_member(role: :administrator, household: household) }
  let(:member) { household_policy_member(role: :member, household: household) }
  let(:grant) { PersonAccessGrant.new(household: household) }

  it 'allows only active owners and administrators to list ambiguous grants' do
    expect(described_class.new(owner.fetch(:context), PersonAccessGrant).index?).to be(true)
    expect(described_class.new(administrator.fetch(:context), PersonAccessGrant).index?).to be(true)
    expect(described_class.new(member.fetch(:context), PersonAccessGrant).index?).to be(false)
  end

  it 'scopes owners and administrators to the active household' do
    foreign_household = create(:household)

    owner_scope = described_class::Scope.new(owner.fetch(:context), PersonAccessGrant.all).resolve
    administrator_scope = described_class::Scope.new(administrator.fetch(:context), PersonAccessGrant.all).resolve

    expect(owner_scope.to_sql).to include('household_id')
    expect(owner_scope.where(household: foreign_household)).to be_empty
    expect(administrator_scope.where(household: foreign_household)).to be_empty
  end

  it 'returns no records for non-admin scopes' do
    expect(described_class::Scope.new(member.fetch(:context), PersonAccessGrant.all).resolve).to be_none
  end
end
