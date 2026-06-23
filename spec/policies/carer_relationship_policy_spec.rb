# frozen_string_literal: true

require 'rails_helper'

RSpec.describe CarerRelationshipPolicy, type: :policy do
  let(:owner) { household_policy_member(role: :owner) }
  let(:household) { owner.fetch(:household) }
  let(:member) { household_policy_member(role: :member, household: household) }
  let(:patient) { household_policy_person(household, person_type: :minor, has_capacity: false) }
  let(:carer) { household_policy_person(household, name: 'Policy Carer') }
  let(:relationship) do
    CarerRelationship.create!(patient: patient, carer: carer, relationship_type: :carer)
  end

  it 'allows household managers to administer relationships in their household' do
    expect(policy_results(owner.fetch(:context), relationship,
                          %i[index? show? update? destroy? activate? assign_dependent?])).to eq(
                            index?: true,
                            show?: true,
                            update?: true,
                            destroy?: true,
                            activate?: true,
                            assign_dependent?: true
                          )
  end

  it 'allows granted members to view and assign dependent relationships without admin actions' do
    expect(described_class.new(member.fetch(:context), relationship).show?).to be(false)

    grant_policy_person_access(member, patient, access_level: :view)

    expect(described_class.new(member.fetch(:context), relationship).show?).to be(true)
    expect(described_class.new(member.fetch(:context), relationship).assign_dependent?).to be(false)

    patient.person_access_grants
           .find_by!(household_membership: member.fetch(:membership))
           .update!(access_level: :manage)

    expect(described_class.new(member.fetch(:context), relationship).assign_dependent?).to be(true)
    expect(described_class.new(member.fetch(:context), relationship).destroy?).to be(false)
  end

  it 'scopes relationships to managers or granted patients' do
    relationship
    other = CarerRelationship.create!(
      patient: household_policy_person(household, person_type: :minor, has_capacity: false),
      carer: carer,
      relationship_type: :carer
    )
    grant_policy_person_access(member, patient, access_level: :view)

    manager_scope = described_class::Scope.new(owner.fetch(:context), CarerRelationship.all).resolve
    member_scope = described_class::Scope.new(member.fetch(:context), CarerRelationship.all).resolve

    expect(manager_scope).to include(relationship, other)
    expect(member_scope).to contain_exactly(relationship)
  end

  def policy_results(context, record, actions)
    actions.index_with { |action| described_class.new(context, record).public_send(action) }
  end
end
