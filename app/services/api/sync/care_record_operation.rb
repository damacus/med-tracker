module Api
  module Sync
    class CareRecordOperation
      include Pundit::Authorization

      RESOURCE_CLASSES = { 'person' => Person, 'health_event' => HealthEvent, 'location' => Location,
                           'medication_review_prompt' => MedicationReviewPrompt }.freeze
      ATTRIBUTES = {
        Person => %w[name date_of_birth email person_type has_capacity],
        HealthEvent => %w[person_id event_kind severity title notes started_on ended_on medication_ids],
        Location => %w[name description],
        MedicationReviewPrompt => MedicationReviews::UpdatePrompt::ATTRIBUTES.map(&:to_s)
      }.freeze

      class Error < StandardError
        attr_reader :code, :status

        def initialize(message, code: 'unprocessable_content', status: :unprocessable_content)
          @code = code
          @status = status
          super(message)
        end
      end

      def initialize(authorization:, household:, request: nil)
        @authorization = authorization
        @household = household
        @request = request
      end

      def call(operation:)
        resource_class = RESOURCE_CLASSES.fetch(operation.fetch(:resource_type))
        action = operation.fetch(:action)
        validate_action!(resource_class, action)
        record = operation_record(resource_class, operation)
        action == 'create' ? create_record(record) : change_record(record, operation)
        record
      rescue ActiveRecord::RecordInvalid, ActiveRecord::RecordNotDestroyed, CareDelegation::Assign::Error, ArgumentError
        raise Error, 'attributes are invalid'
      rescue MedicationReviews::UpdatePrompt::VersionError => e
        raise Error.new(e.message, code: e.code == 'conflict' ? 'sync_conflict' : e.code,
                                   status: e.code == 'conflict' ? :conflict : :precondition_required)
      end

      private

      def operation_record(resource_class, operation)
        return build_record(resource_class, operation) if operation[:action] == 'create'

        find_record(resource_class, operation[:id])
      end

      def pundit_user
        @authorization.with(membership: @authorization.membership.reload)
      end

      def validate_action!(resource_class, action)
        actions = case resource_class.name
                  when 'Person' then %w[create update]
                  when 'MedicationReviewPrompt' then %w[update]
                  else %w[create update delete]
                  end
        return if actions.include?(action)

        raise Error.new('action is unsupported', code: 'sync_operation_unsupported')
      end

      def find_record(resource_class, identifier)
        Api::PortableRecordLocator.new(household: @household).find(
          policy_scope(resource_class).where(household: @household), identifier
        )
      end

      def attributes_for(resource_class, operation)
        attributes = operation.fetch(:attributes, {}).to_h.slice(*ATTRIBUTES.fetch(resource_class))
        return attributes unless resource_class == HealthEvent

        resolve_health_event_attributes(attributes, operation)
      end

      def resolve_health_event_attributes(attributes, operation)
        attributes.delete('person_id') unless operation[:action] == 'create'
        attributes['person_id'] = find_record(Person, attributes['person_id']).id if attributes.key?('person_id')
        if attributes.key?('medication_ids')
          attributes['medication_ids'] = Array(attributes['medication_ids']).map { |id| find_record(Medication, id).id }
        end
        attributes
      end

      def build_record(resource_class, operation)
        resource_class.new(attributes_for(resource_class, operation)).tap { |record| record.household = @household }
      end

      def create_record(record)
        authorize record, :create?
        return record.save! unless record.is_a?(Person)

        People::Create.new(person: record, authorization: pundit_user, request: @request).call
      end

      def change_record(record, operation)
        record.with_lock do
          authorize record, operation[:action] == 'delete' ? :destroy? : :update?
          validate_precondition!(record, operation)
          if operation[:action] == 'delete'
            destroy_record(record)
          else
            update_record(record, operation)
          end
        end
      end

      def update_record(record, operation)
        attributes = attributes_for(record.class, operation)
        return record.update!(attributes) unless record.is_a?(MedicationReviewPrompt)

        MedicationReviews::UpdatePrompt.new(prompt: record, authorization: pundit_user)
                                       .call(attributes: attributes, if_match: operation[:if_match])
      end

      def destroy_record(record)
        if record.is_a?(Location) && MedicationAdministrationHistory.exists_for?(record)
          raise Error, 'delete conflicts with retained administration history'
        end

        record.destroy!
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
