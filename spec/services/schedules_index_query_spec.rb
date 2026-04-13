# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SchedulesIndexQuery do
  describe '#call' do
    it 'returns only active schedules ordered by start_date then id' do
      later_schedule = create(:schedule, start_date: Date.new(2026, 4, 2))
      first_same_day = create(:schedule, start_date: Date.new(2026, 4, 1))
      second_same_day = create(:schedule, start_date: Date.new(2026, 4, 1))
      expired_schedule = create(:schedule, :expired)
      schedule_ids = [later_schedule.id, first_same_day.id, second_same_day.id]

      result = described_class.new(scope: Schedule.where(id: schedule_ids)).call

      expect(result).to eq([first_same_day, second_same_day, later_schedule])
      expect(result.map(&:id)).not_to include(expired_schedule.id)
      expect(result.first.association(:person)).to be_loaded
      expect(result.first.association(:medication)).to be_loaded
    end

    it 'respects the passed scope boundary' do
      included_schedule = create(:schedule)
      create(:schedule)

      result = described_class.new(scope: Schedule.where(id: included_schedule.id)).call

      expect(result).to contain_exactly(included_schedule)
    end
  end
end
