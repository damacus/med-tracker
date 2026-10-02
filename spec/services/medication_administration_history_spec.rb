require 'rails_helper'

RSpec.describe MedicationAdministrationHistory do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules, :person_medications

  %i[schedule person_medication].each do |kind|
    context "with a saved #{kind} decision" do
      let(:source) { kind == :schedule ? schedules(:john_movicol) : person_medications(:john_vitamin_d) }

      before do
        source.medication_dose_occurrences.create!(window_starts_on: Date.current, position: 1, outcome: 'open')
      end

      it 'protects the person, medicine and location while an outcome remains' do
        [source.person, source.medication, source.medication.location].each do |record|
          expect(described_class.exists_for?(record)).to be(true)
        end
      end

      it 'does not treat an unrelated location as having that history' do
        other = Location.create!(household: source.household, name: 'No retained history')
        expect(described_class.exists_for?(other)).to be(false)
      end
    end
  end
end
