module Api
  module Sync
    class AssignmentOperation
      include Pundit::Authorization

      RESOURCE_CLASSES = { 'schedule' => Schedule, 'person_medication' => PersonMedication }.freeze
      COMMON_ATTRIBUTES = %w[
        person_id medication_id dose_amount dose_unit source_dosage_option_id notes
        max_daily_doses min_hours_between_doses dose_cycle
      ].freeze
      SCHEDULE_ATTRIBUTES = %w[frequency start_date end_date schedule_type schedule_config].freeze

      class Error < StandardError
        attr_reader :code, :status

        def initialize(message, code: 'unprocessable_content', status: :unprocessable_content)
          @code = code
          @status = status
          super(message)
        end
      end

      def initialize(authorization:, household:)
        @authorization = authorization
        @household = household
      end

      def call(operation:)
        resource_class = RESOURCE_CLASSES.fetch(operation.fetch(:resource_type))
        case operation.fetch(:action)
        when 'create' then create_record(resource_class, operation)
        when 'update', 'delete' then change_record(resource_class, operation)
        else
          raise Error.new('action is unsupported', code: 'sync_operation_unsupported')
        end
      rescue ActiveRecord::RecordInvalid
        raise Error, 'attributes are invalid'
      end

      private

      attr_reader :household

      def pundit_user = @authorization

      def find_record(resource_class, identifier)
        Api::PortableRecordLocator.new(household: household).find(
          policy_scope(resource_class).where(household: household), identifier
        )
      end

      def create_record(resource_class, operation)
        attributes = permitted_attributes(resource_class, operation)
        person = find_record(Person, attributes.delete('person_id'))
        authorize person, :show?
        record = resource_class.new(person: person, household: household)
        assign_attributes(record, attributes)
        authorize record, :create?
        record.save!
        record
      end

      def change_record(resource_class, operation)
        record = find_record(resource_class, operation.fetch(:id))
        record.with_lock do
          raise ActiveRecord::RecordNotFound if record.retired_at.present?

          authorize record, operation[:action] == 'delete' ? :destroy? : :update?
          validate_precondition!(record, operation)
          apply_change(record, operation)
        end
        record
      end

      def apply_change(record, operation)
        if operation[:action] == 'delete'
          record.retire!
          record.record_sync_deletion!
        else
          attributes = permitted_attributes(record.class, operation).except('person_id')
          assign_attributes(record, attributes)
          record.save!
        end
      end

      def permitted_attributes(resource_class, operation)
        extra = resource_class == Schedule ? SCHEDULE_ATTRIBUTES : %w[administration_kind]
        attributes = operation.fetch(:attributes, {}).to_h.slice(*(COMMON_ATTRIBUTES + extra))
        if attributes['medication_id'].present?
          attributes['medication_id'] = find_record(Medication, attributes['medication_id']).id
        end
        if attributes['source_dosage_option_id'].present?
          attributes['source_dosage_option_id'] = find_record(
            MedicationDosageOption, attributes['source_dosage_option_id']
          ).id
        end
        attributes
      end

      def assign_attributes(record, attributes)
        record.assign_attributes(attributes)
      rescue ArgumentError
        raise Error, 'attributes are invalid'
      end

      def validate_precondition!(record, operation)
        expected = operation[:if_match].to_s
        if expected.blank?
          raise Error.new('if_match is required', code: 'precondition_required', status: :precondition_required)
        end
        return if ActiveSupport::SecurityUtils.secure_compare(expected, Api::RecordEtag.for(record))

        raise Error.new('record has changed since it was last read', code: 'sync_conflict', status: :conflict)
      end
    end
  end
end
