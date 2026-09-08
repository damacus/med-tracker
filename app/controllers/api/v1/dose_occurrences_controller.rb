module Api
  module V1
    class DoseOccurrencesController < BaseController
      ERROR_STATUSES = { 'already_resolved' => :conflict, 'sync_conflict' => :conflict,
                         'precondition_required' => :precondition_required }.freeze

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

      def reopen
        attributes = params.expect(dose_occurrence: [:key])
        record = occurrence_resolver.call(key: attributes[:key], action: 'reopen',
                                          if_match: request.headers['If-Match'])
        render_outcome(record)
      end

      def take
        reject_numeric_contract_values!(%w[dose_amount taken_from_medication_id])
        attributes = params.expect(dose_occurrence: %i[key taken_at client_uuid dose_amount taken_from_medication_id])
        record = occurrence_resolver.take(
          key: attributes[:key], taken_at: Time.iso8601(attributes[:taken_at].to_s),
          if_match: request.headers['If-Match'],
          **attributes.slice(:client_uuid, :dose_amount, :taken_from_medication_id).to_h.symbolize_keys
        )
        render_outcome(record)
      rescue ArgumentError
        render_unprocessable('taken_at must be ISO8601')
      end

      private

      def with_api_idempotency(&)
        if action_name != 'index'
          authorize occurrence_source, action_name == 'reopen' ? :update? : :take_medication?
        end
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
        status = ERROR_STATUSES.fetch(error.code, :unprocessable_content)
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
