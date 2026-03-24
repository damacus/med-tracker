# frozen_string_literal: true

module Api
  module V1
    class PersonMedicationSerializer
      def initialize(person_medication)
        @person_medication = person_medication
      end

      def as_json(*)
        medication_data.merge(dosing_limits)
      end

      private

      attr_reader :person_medication

      def medication_data
        association_data.merge(schedule_data)
      end

      def association_data
        {
          id: person_medication.id,
          person_id: person_medication.person_id,
          medication_id: person_medication.medication_id,
          dose_amount: person_medication.dose_amount&.to_f,
          dose_unit: person_medication.dose_unit
        }
      end

      def schedule_data
        {
          dose_cycle: person_medication.dose_cycle,
          notes: person_medication.notes,
          position: person_medication.position,
          updated_at: person_medication.updated_at.iso8601
        }
      end

      def dosing_limits
        {
          max_daily_doses: person_medication.max_daily_doses,
          min_hours_between_doses: person_medication.min_hours_between_doses
        }
      end
    end
  end
end
