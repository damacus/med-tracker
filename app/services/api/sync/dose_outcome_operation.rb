module Api
  module Sync
    class DoseOutcomeOperation
      include Pundit::Authorization

      ERROR_STATUSES = { 'already_resolved' => :conflict, 'sync_conflict' => :conflict,
                         'precondition_required' => :precondition_required }.freeze

      class Error < StandardError
        attr_reader :code, :status

        def initialize(message, code: 'invalid_occurrence', status: :unprocessable_content)
          @code = code
          @status = status
          super(message)
        end
      end

      def initialize(authorization:, household:)
        @authorization = authorization
        @household = household
      end

      def authorize_operation!(operation)
        source, = context(operation)
        authorize source, operation[:action] == 'update' ? :update? : :take_medication?
      end

      def call(operation:)
        source, record = context(operation)
        resolver = MedicationAdministration::OccurrenceResolver.new(source: source, authorization: pundit_user)
        resolver.call(**resolution_attributes(operation, source, record))
      rescue MedicationAdministration::OccurrenceResolver::Error => e
        raise Error.new(e.message, code: e.code, status: ERROR_STATUSES.fetch(e.code, :unprocessable_content))
      rescue ActiveRecord::RecordInvalid, ArgumentError, TypeError
        raise Error, 'Outcome is invalid'
      end

      private

      def pundit_user = @authorization

      def context(operation)
        attributes = operation.fetch(:attributes, {})
        case [operation[:action], attributes[:outcome]]
        when %w[create not_taken]
          raise Error, 'Source is invalid' unless attributes[:source_type] == 'schedule'

          [find_source(attributes[:source_id]), nil]
        when %w[update open]
          saved_context(operation[:id])
        else
          raise Error.new('Outcome action is unsupported', code: 'sync_operation_unsupported')
        end
      end

      def saved_context(identifier)
        record = locator.find(MedicationDoseOccurrence.where(household: @household), identifier)
        raise ActiveRecord::RecordNotFound unless record.schedule_id

        [find_source(record.schedule_id.to_s), record]
      end

      def resolution_attributes(operation, source, record)
        return { key: occurrence_key(source, record), action: 'reopen', if_match: operation[:if_match] } if record

        attributes = operation.fetch(:attributes, {})
        { key: attributes[:occurrence_key], action: 'not_taken', reason: attributes[:reason].presence,
          note: attributes[:note] }
      end

      def find_source(identifier)
        locator.find(policy_scope(Schedule).where(household: @household), identifier)
      end

      def locator
        Api::PortableRecordLocator.new(household: @household)
      end

      def occurrence_key(source, record)
        rows = MedicationAdministration::OccurrenceProjection.new(
          source: source, start_date: record.window_starts_on, end_date: record.window_starts_on
        ).call
        rows.find { |row| row.position == record.position }&.key
      end
    end
  end
end
