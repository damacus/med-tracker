# frozen_string_literal: true

module Api
  module V1
    class ScheduleSerializer
      def initialize(schedule)
        @schedule = schedule
      end

      def as_json(*)
        schedule_data.merge(dosing_data)
      end

      private

      attr_reader :schedule

      def schedule_data
        association_data.merge(timing_data)
      end

      def association_data
        {
          id: schedule.id,
          person_id: schedule.person_id,
          medication_id: schedule.medication_id,
          dose_amount: schedule.dose_amount,
          dose_unit: schedule.dose_unit,
          frequency: schedule.frequency,
          dose_cycle: schedule.dose_cycle
        }
      end

      def timing_data
        {
          start_date: schedule.start_date&.iso8601,
          end_date: schedule.end_date&.iso8601,
          active: schedule.active?,
          notes: schedule.notes,
          updated_at: schedule.updated_at.iso8601
        }
      end

      def dosing_data
        {
          max_daily_doses: schedule.max_daily_doses,
          min_hours_between_doses: schedule.min_hours_between_doses
        }
      end
    end
  end
end
