# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationTimelineQuery do
  describe '#call' do
    it 'returns related timeline sources for one medication and excludes the passed source' do
      medication = create(:medication)
      other_medication = create(:medication)
      timeline_data = create_timeline_data_for(medication: medication, other_medication: other_medication)

      result = described_class.new(medication: medication, excluding: timeline_data[:schedule]).call

      expect(result.schedules).to contain_exactly(timeline_data[:other_schedule])
      expect(result.person_medications).to contain_exactly(timeline_data[:person_medication])
    end
  end

  def create_timeline_data_for(medication:, other_medication:)
    schedule = create(:schedule, medication: medication)
    other_schedule = create(:schedule, medication: medication)
    create(:schedule, :expired, medication: medication)
    create(:schedule, medication: other_medication)
    person_medication = create(:person_medication, medication: medication)
    create(:person_medication, medication: other_medication)

    {
      schedule: schedule,
      other_schedule: other_schedule,
      person_medication: person_medication
    }
  end
end
