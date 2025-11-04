# frozen_string_literal: true

require 'rails_helper'

RSpec.describe CarerRelationship do
  describe 'associations' do
    it { is_expected.to belong_to(:carer).class_name('Person') }
    it { is_expected.to belong_to(:patient).class_name('Person') }
  end

  describe 'validations' do
    let(:carer) do
      Person.create!(
        name: 'Carer Person',
        date_of_birth: 30.years.ago,
        person_type: :adult
      )
    end

    let(:patient) do
      Person.create!(
        name: 'Patient Person',
        date_of_birth: 5.years.ago,
        person_type: :adult,
        has_capacity: false
      )
    end

    it 'validates uniqueness of carer_id scoped to patient_id' do
      described_class.create!(
        carer: carer,
        patient: patient,
        relationship_type: 'parent'
      )

      duplicate = described_class.new(
        carer: carer,
        patient: patient,
        relationship_type: 'guardian'
      )

      expect(duplicate).not_to be_valid
      expect(duplicate.errors[:carer_id]).to include('has already been taken')
    end

    it 'requires relationship_type' do
      relationship = described_class.new(
        carer: carer,
        patient: patient
      )

      expect(relationship).not_to be_valid
      expect(relationship.errors[:relationship_type]).to include("can't be blank")
    end
  end

  describe 'scopes' do
    let(:carer) do
      Person.create!(
        name: 'Carer Person',
        date_of_birth: 30.years.ago,
        person_type: :adult
      )
    end

    let(:patient) do
      Person.create!(
        name: 'Patient Person',
        date_of_birth: 5.years.ago,
        person_type: :adult
      )
    end

    it 'has an active scope' do
      active_relationship = described_class.create!(
        carer: carer,
        patient: patient,
        relationship_type: 'parent',
        active: true
      )

      inactive_relationship = described_class.create!(
        carer: carer,
        patient: Person.create!(name: 'Another Patient', date_of_birth: 10.years.ago),
        relationship_type: 'guardian',
        active: false
      )

      expect(described_class.active).to include(active_relationship)
      expect(described_class.active).not_to include(inactive_relationship)
    end
  end

  describe '#deactivate!' do
    it 'sets active to false' do
      relationship = described_class.create!(
        carer: Person.create!(name: 'Carer', date_of_birth: 30.years.ago, person_type: :adult),
        patient: Person.create!(name: 'Patient', date_of_birth: 5.years.ago, person_type: :adult),
        relationship_type: 'parent',
        active: true
      )

      relationship.deactivate!

      expect(relationship.reload.active).to be false
    end
  end

  describe '#activate!' do
    it 'sets active to true' do
      relationship = described_class.create!(
        carer: Person.create!(name: 'Carer', date_of_birth: 30.years.ago, person_type: :adult),
        patient: Person.create!(name: 'Patient', date_of_birth: 5.years.ago, person_type: :adult),
        relationship_type: 'parent',
        active: false
      )

      relationship.activate!

      expect(relationship.reload.active).to be true
    end
  end
end
