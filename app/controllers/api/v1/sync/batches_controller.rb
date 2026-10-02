# frozen_string_literal: true

module Api
  module V1
    module Sync
      class BatchesController < Api::V1::BaseController
        class BatchError < StandardError
          attr_reader :code, :status

          def initialize(message, code: 'unprocessable_content', status: :unprocessable_content)
            @code = code
            @status = status
            super(message)
          end
        end

        class PreconditionRequired < BatchError; end
        class SyncConflict < BatchError; end

        rescue_from Api::Sync::DoseOutcomeOperation::Error, with: :render_outcome_error
        rescue_from Api::Sync::CareRecordOperation::Error, with: :render_outcome_error

        def create
          results = Households::LifecycleCutoffLock.with(household: current_household) do
            apply_batch_with_retry
          end

          render json: { data: { applied: true, results: results } }, status: :created
        rescue PreconditionRequired => e
          render_api_error(code: 'precondition_required', message: e.message, status: :precondition_required)
        rescue SyncConflict => e
          render_conflict(e.message, code: 'sync_conflict')
        rescue BatchError => e
          render_api_error(code: e.code, message: e.message, status: e.status)
        end

        private

        def authorize_pause_replay!
          operations.each do |operation|
            next unless operation[:resource_type] == 'medication_pause_period'

            medication_pause_period_operation.authorize_replay!(operation: operation)
          end
        end

        def with_api_idempotency(&)
          operations.each do |operation|
            Api::Sync::OperationCatalog.validate!(operation)
            outcome_operation.authorize_operation!(operation) if outcome_operation?(operation)
          end
          super
        end

        def authorize_api_replay!(record)
          return unless record.response_status.between?(200, 299)

          authorize_pause_replay!
          Api::Sync::ReplayAuthorization.new(authorization: pundit_user, household: current_household)
                                        .call(operations: operations, results: record.response_body.dig('data', 'results'))
        end

        def outcome_operation?(operation)
          operation[:resource_type] == 'medication_dose_occurrence'
        end

        def outcome_operation
          Api::Sync::DoseOutcomeOperation.new(authorization: pundit_user, household: current_household)
        end

        def render_outcome_error(error)
          render_api_error(code: error.code, message: error.message, status: error.status)
        end

        def apply_batch_with_retry
          retries = 0

          begin
            apply_batch
          rescue Api::Sync::MedicationTakeOperation::RetryBatch => e
            retries += 1
            retry if retries == 1

            unless e.client_uuid_constraint?
              raise BatchError.new(
                'Medication take is invalid',
                code: 'medication_take_invalid'
              )
            end

            raise BatchError.new(
              'Medication take idempotency key is unavailable',
              code: 'idempotency_key_unavailable',
              status: :conflict
            )
          end
        end

        def apply_batch
          results = []
          ActiveRecord::Base.transaction(requires_new: true) do
            operations.each_with_index do |operation, index|
              results << apply_operation(operation, index)
            end
          end

          results
        end

        def operations
          reject_numeric_contract_values!(%w[
                                            id source_id person_id medication_id source_dosage_option_id dose_amount
                                            current_supply reorder_threshold min_hours_between_doses amount
                                            medication_ids location_id quantity new_quantity dosage_id
                                            default_min_hours_between_doses
                                          ])
          params.expect(batch: [{ operations: [[:action, :resource_type, :id, :if_match, { attributes: {} }]] }])
                .fetch(:operations)
        end

        def apply_operation(operation, index)
          if operation[:resource_type] == 'medication_pause_period'
            return apply_medication_pause_period_operation(operation, index)
          end

          if Api::Sync::InventoryOperation::RESOURCE_CLASSES.key?(operation[:resource_type])
            return apply_inventory_operation(operation, index)
          end
          if Api::Sync::CareRecordOperation::RESOURCE_CLASSES.key?(operation[:resource_type])
            return apply_care_operation(operation, index)
          end

          if outcome_operation?(operation)
            record = outcome_operation.call(operation: operation)
            return batch_result(record, index, operation[:action]).merge(etag: api_etag(record))
          end

          if Api::Sync::AssignmentOperation::RESOURCE_CLASSES.key?(operation[:resource_type])
            return apply_assignment_operation(operation, index)
          end

          reject_medication_take_mutation!(operation, index)

          return create_record(operation, index) if operation.fetch(:action) == 'create'

          raise BatchError, "operation #{index} action is unsupported"
        end

        def apply_medication_pause_period_operation(operation, index)
          result = medication_pause_period_operation.call(operation: operation)
          batch_result(result.period, index, operation.fetch(:action)).merge(
            etag: api_etag(result.period), replayed: result.replayed
          )
        rescue Api::Sync::MedicationPausePeriodOperation::Error => e
          raise BatchError.new("operation #{index} #{e.message}", code: e.code, status: e.status)
        end

        def apply_care_operation(operation, index)
          record = Api::Sync::CareRecordOperation.new(authorization: pundit_user, household: current_household,
                                                      request: request).call(operation: operation)
          result = { index: index, action: operation[:action], record_type: record.class.name, record_id: record.id.to_s }
          result[:record_portable_id] = record.portable_id if record.respond_to?(:portable_id)
          operation[:action] == 'delete' ? result : result.merge(etag: api_etag(record))
        end

        def apply_inventory_operation(operation, index)
          record = Api::Sync::InventoryOperation.new(authorization: pundit_user, household: current_household)
                                                .call(operation: operation)
          result = batch_result(record, index, operation[:action])
          operation[:action] == 'delete' ? result : result.merge(etag: api_etag(record))
        rescue Api::Sync::InventoryOperation::Error => e
          raise BatchError.new("operation #{index} #{e.message}", code: e.code, status: e.status)
        end

        def apply_assignment_operation(operation, index)
          record = Api::Sync::AssignmentOperation.new(
            authorization: pundit_user, household: current_household
          ).call(operation: operation)
          result = batch_result(record, index, operation.fetch(:action))
          operation[:action] == 'delete' ? result : result.merge(etag: api_etag(record))
        rescue Api::Sync::AssignmentOperation::Error => e
          raise BatchError.new("operation #{index} #{e.message}", code: e.code, status: e.status)
        rescue KeyError
          raise BatchError, "operation #{index} resource_type, action and id are required"
        end

        def reject_medication_take_mutation!(operation, index)
          return unless operation[:resource_type] == 'medication_take'
          return if operation[:action] == 'create'

          raise BatchError.new(
            "operation #{index} action is unsupported",
            code: 'sync_operation_unsupported'
          )
        end

        def create_record(operation, index)
          unless operation[:resource_type] == 'medication_take'
            raise BatchError.new(
              "operation #{index} action is unsupported",
              code: 'sync_operation_unsupported'
            )
          end

          attributes = operation.fetch(:attributes, {})
          existing_take = idempotent_medication_take(attributes[:client_uuid])
          result = medication_take_operation.call(
            attributes: attributes,
            existing_take: existing_take,
            user: current_user,
            authorization: pundit_user,
            route: request.path
          ) do |source_type, source_id|
            medication_take_source(source_type, source_id).tap do |source|
              authorize source, :take_medication?
            end
          end

          batch_result(result.take, index, 'create').merge(replayed: result.replayed)
        rescue Api::Sync::MedicationTakeOperation::Error => e
          raise BatchError.new("operation #{index} #{e.message}", code: e.code, status: e.status)
        end

        def idempotent_medication_take(client_uuid)
          return if client_uuid.blank?

          policy_scope(MedicationTake).find_by(client_uuid: client_uuid).tap do |take|
            authorize take, :create? if take
          end
        end

        def medication_take_source(source_type, source_id)
          case source_type
          when 'schedule'
            find_api_record(policy_scope(Schedule), source_id)
          when 'person_medication'
            find_api_record(policy_scope(PersonMedication), source_id)
          else
            raise ActiveRecord::RecordNotFound
          end
        end

        def medication_take_operation
          @medication_take_operation ||= Api::Sync::MedicationTakeOperation.new
        end

        def medication_pause_period_operation
          @medication_pause_period_operation ||= Api::Sync::MedicationPausePeriodOperation.new(
            authorization: pundit_user, household: current_household, membership: current_membership
          )
        end

        def batch_result(record, index, action)
          {
            index: index,
            action: action,
            record_type: record.class.name,
            record_portable_id: record.portable_id
          }
        end
      end
    end
  end
end
