# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DependentRelationshipAssigner do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :carer_relationships

  describe '#call' do
    it 'does not create duplicate relationships for active assignments' do
      carer = users(:jane).person
      dependent = people(:child_patient)

      expect do
        described_class.new(
          carer: carer,
          dependent_ids: [dependent.id],
          relationship_type: 'parent'
        ).call
      end.not_to change(CarerRelationship, :count)
    end

    it 'reactivates inactive relationships instead of creating duplicates' do
      carer = users(:bob).person
      dependent = people(:child_patient)
      relationship = inactive_relationship(carer, dependent)

      expect do
        described_class.new(
          carer: carer,
          dependent_ids: [dependent.id],
          relationship_type: 'parent'
        ).call
      end.not_to change(CarerRelationship, :count)

      expect(relationship.reload).to have_attributes(
        relationship_type: 'parent',
        active: true
      )
    end

    it 'ignores adults and unrelated ids' do
      carer = users(:parent).person

      expect do
        described_class.new(
          carer: carer,
          dependent_ids: [people(:john).id, nil, ''],
          relationship_type: 'parent'
        ).call
      end.not_to change(CarerRelationship, :count)
    end

    it 'ignores non-numeric ids' do
      carer = users(:parent).person

      expect do
        described_class.new(
          carer: carer,
          dependent_ids: ['not-an-id'],
          relationship_type: 'parent'
        ).call
      end.not_to change(CarerRelationship, :count)
    end
  end

  def inactive_relationship(carer, dependent)
    CarerRelationship.create!(
      carer: carer,
      patient: dependent,
      relationship_type: 'family_member',
      active: false
    )
  end
end
