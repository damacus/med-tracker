module Api
  module Sync
    class MedicationPausePeriodOperation
      include Pundit::Authorization

      SOURCE_CLASSES = { 'schedule' => Schedule, 'person_medication' => PersonMedication }.freeze
      Result = Data.define(:period, :replayed)

      class Error < StandardError
        attr_reader :code, :status

        def initialize(message, code: 'unprocessable_content', status: :unprocessable_content)
          @code = code
          @status = status
          super(message)
        end
      end

      def initialize(authorization:, household:, membership:)
        @authorization = authorization
        @household = household
        @membership = membership
      end

      def call(operation:)
        case operation.fetch(:action)
        when 'create' then create_period(operation)
        when 'close' then close_period(operation)
        else
          raise Error.new('action is unsupported', code: 'sync_operation_unsupported')
        end
      rescue KeyError
        raise Error, 'resource_type, action and required attributes are required'
      rescue ActiveRecord::RecordInvalid
        raise Error, 'attributes are invalid'
      end

      def authorize_replay!(operation:)
        case operation.fetch(:action)
        when 'create'
          attributes = create_attributes(operation)
          authorize find_source(attributes.fetch('source_type'), attributes.fetch('source_id')), :update?
        when 'close'
          period = visible_periods.find_by!(portable_id: operation.fetch(:id))
          authorize period.schedule || period.person_medication, :update?
        end
      end

      private

      attr_reader :household, :membership

      def pundit_user = @authorization

      def create_period(operation)
        attributes = create_attributes(operation)
        source = find_source(attributes.fetch('source_type'), attributes.fetch('source_id'))
        authorize source, :update?
        pause_source(source, attributes)
      end

      def pause_source(source, attributes)
        source.with_lock do
          existing_period = source.medication_pause_periods.exists?(ended_at: nil)
          period_service = MedicationAdministration::PausePeriodService.new(
            source:, membership:, reason: attributes.fetch('reason'), note: attributes['note'], started_at: Time.current
          )
          Result.new(period: period_service.call, replayed: existing_period)
        end
      end

      def close_period(operation)
        raise Error, 'attributes are invalid' if operation.fetch(:attributes, {}).to_h.present?

        period = visible_periods.find_by!(portable_id: operation.fetch(:id))
        source = period.schedule || period.person_medication
        authorize source, :update?
        close_source_period(source, period, operation)
      end

      def close_source_period(source, period, operation)
        source.with_lock do
          period = source.medication_pause_periods.find(period.id)
          validate_precondition!(period, operation)
          replayed = period.ended_at?
          period_service = MedicationAdministration::ResumePeriodService.new(
            source:, membership:, ended_at: Time.current, period:
          )
          Result.new(period: period_service.call, replayed:)
        end
      end

      def find_source(source_type, identifier)
        unless identifier.is_a?(String) && identifier.match?(Api::PortableRecordLocator::UUID_PATTERN)
          raise ActiveRecord::RecordNotFound
        end

        source_class = SOURCE_CLASSES[source_type]
        raise ActiveRecord::RecordNotFound unless source_class

        Api::PortableRecordLocator.new(household:).find(
          policy_scope(source_class).where(household:), identifier
        )
      end

      def visible_periods
        scope = MedicationPausePeriod.where(household:)
        scope.where(schedule_id: policy_scope(Schedule).select(:id))
             .or(scope.where(person_medication_id: policy_scope(PersonMedication).select(:id)))
      end

      def validate_context!(attributes)
        valid_reason = MedicationPausePeriod::PUBLIC_REASONS.include?(attributes['reason'])
        valid_note = !attributes.key?('note') || attributes['note'].nil? || attributes['note'].is_a?(String)
        raise Error, 'a supported pause reason is required' unless valid_reason && valid_note
      end

      def create_attributes(operation)
        operation.fetch(:attributes, {}).to_h.stringify_keys.tap do |attributes|
          validate_create_attributes!(attributes)
          validate_context!(attributes)
        end
      end

      def validate_create_attributes!(attributes)
        permitted = %w[source_type source_id reason note]
        raise Error, 'attributes are invalid' if (attributes.keys - permitted).any?
      end

      def validate_precondition!(period, operation)
        expected = operation[:if_match].to_s
        if expected.blank?
          raise Error.new('if_match is required', code: 'precondition_required', status: :precondition_required)
        end
        return if ActiveSupport::SecurityUtils.secure_compare(expected, Api::RecordEtag.for(period))

        raise Error.new('record has changed since it was last read', code: 'sync_conflict', status: :conflict)
      end
    end
  end
end
