# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationWorkflowPeopleQuery do
  describe '#call' do
    it 'returns people in name order filtered by the injected predicate' do
      beta_person = create(:person, name: 'Beta Person')
      alpha_person = create(:person, name: 'Alpha Person')
      gamma_person = create(:person, name: 'Gamma Person')

      result = described_class.new(
        people_scope: Person.where(id: [beta_person.id, alpha_person.id, gamma_person.id]),
        preload_person: nil,
        can_add_medication: ->(person) { person.name != 'Gamma Person' }
      ).call

      expect(result.map(&:name)).to eq(['Alpha Person', 'Beta Person'])
    end

    it 'respects the passed scope boundary' do
      included_person = create(:person)
      create(:person)

      result = described_class.new(
        people_scope: Person.where(id: included_person.id),
        preload_person: nil,
        can_add_medication: ->(_person) { true }
      ).call

      expect(result).to contain_exactly(included_person)
    end

    it 'preloads patients when a preload person is provided' do
      patients = instance_double(ActiveRecord::Associations::CollectionProxy)
      preload_person = instance_double(Person, patients: patients)

      allow(patients).to receive(:load)

      described_class.new(
        people_scope: Person.none,
        preload_person: preload_person,
        can_add_medication: ->(_person) { true }
      ).call

      expect(patients).to have_received(:load)
    end
  end
end
