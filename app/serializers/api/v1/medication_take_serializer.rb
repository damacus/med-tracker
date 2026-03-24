# frozen_string_literal: true

module Api
  module V1
    class MedicationTakeSerializer
      def initialize(medication_take)
        @medication_take = medication_take
      end

      def as_json(*)
        take_data.merge(subject_data)
      end

      private

      attr_reader :medication_take

      def take_data
        source_data.merge(event_data)
      end

      def source_data
        {
          id: medication_take.id,
          schedule_id: medication_take.schedule_id,
          person_medication_id: medication_take.person_medication_id,
          taken_from_medication_id: medication_take.taken_from_medication_id,
          taken_from_location_id: medication_take.taken_from_location_id
        }
      end

      def event_data
        {
          amount_ml: medication_take.amount_ml&.to_f,
          taken_at: medication_take.taken_at&.iso8601,
          updated_at: medication_take.updated_at.iso8601
        }
      end

      def subject_data
        {
          person_id: medication_take.person&.id,
          medication_id: medication_take.medication&.id
        }
      end
    end
  end
end
