module Api
  module Sync
    class ReplayAuthorization
      RESOURCE_CLASSES = CareRecordOperation::RESOURCE_CLASSES.merge(InventoryOperation::RESOURCE_CLASSES)
                                                              .merge(AssignmentOperation::RESOURCE_CLASSES)
                                                              .merge('medication_take' => MedicationTake).freeze

      def initialize(authorization:, household:)
        @authorization = authorization
        @household = household
      end

      def call(operations:, results:)
        Households::LifecycleCutoffLock.with(household: @household) do
          context = @authorization.with(membership: @authorization.membership.reload)
          results.zip(operations).each do |result, operation|
            authorize_result!(context, operation, result)
          end
        end
      end

      private

      def authorize_result!(context, operation, result)
        return if %w[medication_dose_occurrence medication_pause_period].include?(operation[:resource_type])

        resource_class = RESOURCE_CLASSES.fetch(operation[:resource_type])
        record = find_result(resource_class, operation, result)
        Pundit.authorize(context, record, policy_action(operation[:action]))
        Pundit.authorize(context, record, :show?) if operation[:action] == 'create' && record.is_a?(Person)
      end

      def find_result(resource_class, operation, result)
        scope = resource_class.where(household: @household)
        if resource_class == MedicationReviewPrompt
          scope.find(result.fetch('record_id'))
        else
          scope.find_by!(portable_id: result.fetch('record_portable_id'))
        end
      rescue ActiveRecord::RecordNotFound
        raise unless operation[:action] == 'delete'

        deleted_result(resource_class, result)
      end

      def deleted_result(resource_class, result)
        tombstone = ApiTombstone.find_by!(household: @household, record_type: resource_class.name,
                                          record_portable_id: result.fetch('record_portable_id'))
        case resource_class.name
        when 'Medication', 'Location'
          resource_class.new(household: @household)
        when 'HealthEvent'
          identifier = tombstone.metadata['person_portable_id'].presence || raise(ActiveRecord::RecordNotFound)
          person = @household.people.find_by!(portable_id: identifier)
          HealthEvent.new(household: @household, person: person)
        else
          raise ActiveRecord::RecordNotFound
        end
      end

      def policy_action(action)
        case action
        when 'create' then :create?
        when 'delete' then :destroy?
        when 'mark_as_ordered' then :mark_as_ordered?
        when 'mark_as_received' then :mark_as_received?
        else :update?
        end
      end
    end
  end
end
