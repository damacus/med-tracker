# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ScheduleWorkflowQuery do
  describe '#options' do
    it 'returns people and medications in name order' do
      beta_person = create(:person, name: 'Beta Person')
      alpha_person = create(:person, name: 'Alpha Person')
      zeta_medication = create(:medication, name: 'Zeta Medication')
      alpha_medication = create(:medication, name: 'Alpha Medication')

      result = described_class.new(
        people_scope: Person.where(id: [beta_person.id, alpha_person.id]),
        medications_scope: Medication.where(id: [zeta_medication.id, alpha_medication.id])
      ).options

      expect(result.people.map(&:name)).to eq(['Alpha Person', 'Beta Person'])
      expect(result.medications.map(&:name)).to eq(['Alpha Medication', 'Zeta Medication'])
      expect(result.medications.first.association(:location)).to be_loaded
    end
  end

  describe '#selection' do
    it 'resolves the selected records from the provided scopes only' do
      allowed_person = create(:person)
      allowed_medication = create(:medication)
      create(:person)
      create(:medication)

      result = described_class.new(
        people_scope: Person.where(id: allowed_person.id),
        medications_scope: Medication.where(id: allowed_medication.id)
      ).selection(person_id: allowed_person.id, medication_id: allowed_medication.id)

      expect(result.person).to eq(allowed_person)
      expect(result.medication).to eq(allowed_medication)
    end

    it 'raises when the selected person is outside the provided scope' do
      out_of_scope_person = create(:person)
      medication = create(:medication)

      query = described_class.new(
        people_scope: Person.none,
        medications_scope: Medication.where(id: medication.id)
      )

      expect do
        query.selection(person_id: out_of_scope_person.id, medication_id: medication.id)
      end.to raise_error(ActiveRecord::RecordNotFound)
    end

    it 'raises when the selected medication is outside the provided scope' do
      person = create(:person)
      out_of_scope_medication = create(:medication)

      query = described_class.new(
        people_scope: Person.where(id: person.id),
        medications_scope: Medication.none
      )

      expect do
        query.selection(person_id: person.id, medication_id: out_of_scope_medication.id)
      end.to raise_error(ActiveRecord::RecordNotFound)
    end
  end
end
