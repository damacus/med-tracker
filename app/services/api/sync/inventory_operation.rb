module Api
  module Sync
    class InventoryOperation
      include Pundit::Authorization

      RESOURCE_CLASSES = { 'medication' => Medication, 'medication_dosage_option' => MedicationDosageOption }.freeze
      MEDICATION_ATTRIBUTES = %w[
        name friendly_name barcode dmd_code dmd_system dmd_concept_class category description dose_amount dose_unit
        current_supply reorder_threshold warnings location_id default_schedule_type
      ].freeze
      DOSAGE_ATTRIBUTES = %w[
        medication_id amount unit frequency description default_for_adults default_for_children default_max_daily_doses
        default_min_hours_between_doses default_dose_cycle current_supply reorder_threshold
      ].freeze
      ACTIONS = %w[create update delete adjust_inventory mark_as_ordered mark_as_received remove_stock].freeze
      Error = CareRecordOperation::Error

      def initialize(authorization:, household:)
        @authorization = authorization
        @household = household
      end

      def call(operation:)
        resource_class = RESOURCE_CLASSES.fetch(operation.fetch(:resource_type))
        validate_action!(resource_class, operation[:action])
        return create_record(resource_class, operation) if operation[:action] == 'create'

        change_record(resource_class, operation)
      rescue ActiveRecord::RecordInvalid, ActiveRecord::RecordNotDestroyed, ArgumentError
        raise Error, 'inventory attributes are invalid'
      end

      private

      def pundit_user
        @authorization.with(membership: @authorization.membership.reload)
      end

      def validate_action!(resource_class, action)
        allowed = resource_class == Medication ? ACTIONS : %w[create update]
        return if allowed.include?(action)

        raise Error.new('action is unsupported', code: 'sync_operation_unsupported')
      end

      def find_record(resource_class, identifier)
        Api::PortableRecordLocator.new(household: @household).find(
          policy_scope(resource_class).where(household: @household), identifier
        )
      end

      def attributes_for(resource_class, operation)
        keys = resource_class == Medication ? MEDICATION_ATTRIBUTES : DOSAGE_ATTRIBUTES
        attributes = operation.fetch(:attributes, {}).to_h.slice(*keys)
        resolve_references(resource_class, operation, attributes)
      end

      def resolve_references(resource_class, operation, attributes)
        if resource_class == Medication
          if attributes['location_id'].present?
            attributes['location_id'] =
              find_record(Location, attributes['location_id']).id
          end
        elsif operation[:action] == 'create'
          attributes['medication_id'] = find_record(Medication, attributes['medication_id']).id
        else
          attributes.delete('medication_id')
        end
        attributes
      end

      def create_record(resource_class, operation)
        record = resource_class.new(attributes_for(resource_class, operation))
        record.household = @household
        if record.is_a?(Medication)
          record.created_by_membership_id = @authorization.membership.id
          record.paper_trail_event = 'api_create'
        end
        authorize record, :create?
        record.save!
        record
      end

      def change_record(resource_class, operation)
        record = find_record(resource_class, operation[:id])
        if operation[:action] == 'remove_stock'
          authorize record, :update?
          remove_stock(record, operation.fetch(:attributes, {}).to_h.symbolize_keys)
          return record
        end

        record.with_lock do
          authorize record, policy_action(operation[:action])
          validate_precondition!(record, operation)
          apply_change(record, operation)
        end
        record
      end

      def policy_action(action)
        return :destroy? if action == 'delete'
        return :mark_as_ordered? if action == 'mark_as_ordered'
        return :mark_as_received? if action == 'mark_as_received'

        :update?
      end

      def apply_change(record, operation)
        attributes = operation.fetch(:attributes, {}).to_h.symbolize_keys
        case operation[:action]
        when 'update' then update_record(record, operation)
        when 'delete' then destroy_record(record)
        when 'adjust_inventory' then adjust_inventory(record, attributes)
        when 'mark_as_ordered' then set_order_status(record, :ordered, attributes)
        when 'mark_as_received' then set_order_status(record, :received, attributes)
        when 'remove_stock' then remove_stock(record, attributes)
        end
      end

      def update_record(record, operation)
        record.paper_trail_event = 'api_update' if record.is_a?(Medication)
        record.update!(attributes_for(record.class, operation))
      end

      def destroy_record(record)
        if MedicationAdministrationHistory.exists_for?(record)
          raise Error,
                'delete conflicts with retained administration history'
        end

        record.destroy!
      end

      def adjust_inventory(record, attributes)
        result = AdjustMedicationInventoryService.new.call(medication: record,
                                                           **attributes.slice(:new_quantity, :reason))
        raise Error, 'inventory adjustment is invalid' unless result.success?
      end

      def set_order_status(record, status, attributes)
        details = attributes.slice(:supplier, :quantity, :expected_arrival_on)
        MedicationReorderStatusService.new.call(medication: record, status: status,
                                                order_details: details)
      end

      def remove_stock(record, attributes)
        raise Error, 'stock removal is invalid' unless attributes[:quantity].to_s.match?(/\A[0-9]+(?:\.[0-9]{1,2})?\z/)

        result = RemoveMedicationStockService.new.call(
          medication: record, **attributes.slice(:quantity, :reason, :note, :dosage_id, :submission_id)
        )
        raise Error, 'stock removal is invalid' unless result.success?
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
