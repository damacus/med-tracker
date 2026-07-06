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
        take_identity_data.merge(dose_source_data).merge(inventory_source_data)
      end

      def take_identity_data
        {
          id: medication_take.id,
          portable_id: medication_take.portable_id,
          client_uuid: medication_take.client_uuid
        }
      end

      def dose_source_data
        {
          schedule_id: medication_take.schedule_id,
          schedule_portable_id: medication_take.schedule&.portable_id,
          person_medication_id: medication_take.person_medication_id,
          person_medication_portable_id: medication_take.person_medication&.portable_id
        }
      end

      def inventory_source_data
        {
          taken_from_medication_id: medication_take.taken_from_medication_id,
          taken_from_medication_portable_id: medication_take.taken_from_medication&.portable_id,
          taken_from_location_id: medication_take.taken_from_location_id,
          taken_from_location_portable_id: medication_take.taken_from_location&.portable_id
        }
      end

      def event_data
        {
          dose_amount: medication_take.dose_amount&.to_f,
          dose_unit: medication_take.dose_unit,
          taken_at: medication_take.taken_at&.iso8601,
          updated_at: medication_take.updated_at.iso8601
        }
      end

      def subject_data
        {
          person_id: medication_take.person&.id,
          person_portable_id: medication_take.person&.portable_id,
          medication_id: medication_take.medication&.id,
          medication_portable_id: medication_take.medication&.portable_id
        }
      end
    end
  end
end
