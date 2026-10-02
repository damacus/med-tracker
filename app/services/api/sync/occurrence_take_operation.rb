module Api
  module Sync
    class OccurrenceTakeOperation
      def call(attributes:, authorization:, existing_take: nil, route: nil)
        source = yield(attributes[:source_type], attributes[:source_id])
        validate_replay!(source, attributes, existing_take) if existing_take
        record = resolve(source, attributes, authorization, route)
        MedicationTakeOperation::Result.new(take: record.medication_take, replayed: existing_take.present?)
      rescue MedicationAdministration::OccurrenceResolver::Error => e
        raise MedicationTakeOperation::Error.new(code: e.code, message: e.message, status: error_status(e.code))
      rescue ActiveRecord::RecordInvalid, ArgumentError, TypeError
        raise invalid_error
      end

      private

      def resolve(source, attributes, authorization, route)
        MedicationAdministration::OccurrenceResolver.new(source: source, authorization: authorization).take(
          key: attributes[:occurrence_key], taken_at: parsed_time(attributes[:taken_at]),
          client_uuid: attributes[:client_uuid], dose_amount: attributes[:dose_amount],
          taken_from_medication_id: attributes[:taken_from_medication_id],
          if_match: attributes[:occurrence_etag], route: route
        )
      end

      def parsed_time(value)
        Time.zone.parse(value.to_s) || raise(invalid_error)
      end

      def validate_replay!(source, attributes, take)
        record = linked_record!(source, take)
        window = record.window_starts_on..(record.window_ends_on || record.window_starts_on)
        raise invalid_error unless window.cover?(parsed_time(attributes[:taken_at]).to_date)

        identity = replay_identity(source, record)
        return if MedicationAdministration::OccurrenceProjection.decode(attributes[:occurrence_key]) == identity

        raise invalid_error
      end

      def linked_record!(source, take)
        record = MedicationDoseOccurrence.find_by(household: source.household, medication_take: take)
        type = MedicationDoseSource.new(source).type
        return record if record && record.public_send("#{type}_id") == source.id

        raise invalid_error
      end

      def replay_identity(source, record)
        [MedicationDoseSource.new(source).type, source.portable_id, record.window_starts_on.iso8601, record.position]
      end

      def error_status(code)
        return :conflict if %w[already_resolved sync_conflict].include?(code)
        return :precondition_required if code == 'precondition_required'

        :unprocessable_content
      end

      def invalid_error
        MedicationTakeOperation::Error.new(code: 'invalid_occurrence', message: 'Dose occurrence is invalid')
      end
    end
  end
end
