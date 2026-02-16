# frozen_string_literal: true

require 'rails_helper'

RSpec.describe FamilyDashboard::ScheduleQuery do
  fixtures :people, :carer_relationships, :prescriptions, :person_medicines, :medication_takes, :medicines, :dosages

  let(:person) { people(:jane) }
  let(:child) { people(:child_patient) }
  let(:query) { described_class.new(person) }

  describe '#call' do
    it 'returns an aggregated list of doses for the person and their dependents' do
      results = query.call
      expect(results).to be_an(Array)
      # Jane should have her own doses and child_patient's doses
      people_in_results = results.pluck(:person).uniq
      expect(people_in_results).to include(person, child)
    end

    it 'includes doses from both prescriptions and person_medicines' do
      # This assumes our fixtures have both for the person/child
      results = query.call
      source_types = results.map { |r| r[:source].class.name }.uniq
      expect(source_types).to include('Prescription', 'PersonMedicine')
    end

    it 'correctly identifies taken doses' do
      # We'll need to ensure a MedicationTake exists in the test/fixtures
      results = query.call
      taken_doses = results.select { |r| r[:status] == :taken }
      expect(taken_doses).not_to be_empty
    end
  end
end
