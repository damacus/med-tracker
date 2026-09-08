module Api
  module V1
    class DoseOccurrencesController < BaseController
      rescue_from MedicationAdministration::OccurrenceResolver::Error, with: :render_occurrence_error
      rescue_from ActiveRecord::RecordInvalid, with: :render_invalid_outcome

      def index
        source = find_api_record(policy_scope(Schedule), params.expect(:schedule_id))
        authorize source, :show?
        rows = MedicationAdministration::OccurrenceProjection.new(
          source: source, start_date: occurrence_date(:start_date), end_date: occurrence_date(:end_date)
        ).call
        render json: { data: rows.map { |row| DoseOccurrenceSerializer.new(row).as_json } }
      rescue ArgumentError
        render_unprocessable('A valid date range of at most 31 days is required')
      end

      def not_taken
        attributes = params.expect(dose_occurrence: %i[key reason note])
        record = occurrence_resolver.call(key: attributes[:key], action: 'not_taken',
                                          reason: attributes[:reason], note: attributes[:note])
        render_outcome(record)
      end

      private

      def with_api_idempotency(&)
        authorize occurrence_source, :take_medication? unless action_name == 'index'
        super
      end

      def occurrence_source
        @occurrence_source ||= find_api_record(policy_scope(Schedule), params.expect(:schedule_id))
      end

      def occurrence_resolver
        MedicationAdministration::OccurrenceResolver.new(source: occurrence_source, authorization: pundit_user)
      end

      def render_outcome(record)
        rows = MedicationAdministration::OccurrenceProjection.new(
          source: occurrence_source, start_date: record.window_starts_on, end_date: record.window_starts_on
        ).call
        row = rows.find { |occurrence| occurrence.position == record.position }
        response.set_header('ETag', api_etag(record))
        render json: { data: DoseOccurrenceSerializer.new(row).as_json }
      end

      def render_occurrence_error(error)
        status = error.code == 'already_resolved' ? :conflict : :unprocessable_content
        render_api_error(code: error.code, message: error.message, status: status)
      end

      def render_invalid_outcome
        render_api_error(code: 'validation_failed', message: 'Outcome is invalid', status: :unprocessable_content)
      end

      def occurrence_date(name)
        value = params[name].to_s
        raise ArgumentError unless value.match?(/\A\d{4}-\d{2}-\d{2}\z/)

        Date.iso8601(value)
      end
    end
  end
end
