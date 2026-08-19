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

        def create
          results = apply_batch_with_retry

          render json: { data: { applied: true, results: results } }, status: :created
        rescue PreconditionRequired => e
          render_api_error(code: 'precondition_required', message: e.message, status: :precondition_required)
        rescue SyncConflict => e
          render_conflict(e.message, code: 'sync_conflict')
        rescue BatchError => e
          render_api_error(code: e.code, message: e.message, status: e.status)
        end

        private

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
          reject_numeric_contract_values!(%w[id source_id dose_amount current_supply reorder_threshold])
          params.expect(batch: [{ operations: [[:action, :resource_type, :id, :if_match, { attributes: {} }]] }])
                .fetch(:operations)
        end

        def apply_operation(operation, index)
          reject_medication_take_mutation!(operation, index)

          case operation.fetch(:action)
          when 'create'
            create_record(operation, index)
          when 'update'
            update_record(operation, index)
          when 'delete'
            delete_record(operation, index)
          else
            raise BatchError, "operation #{index} action is unsupported"
          end
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

        def update_record(operation, index)
          record = find_batch_record(operation)
          authorize record, :update?
          record.with_lock do
            validate_precondition!(record, operation, index)
            attributes = permitted_attributes_for(record, operation.fetch(:attributes, {}))
            raise BatchError, "operation #{index} attributes are invalid" unless record.update(attributes)

            batch_result(record, index, 'update').merge(etag: api_etag(record))
          end
        end

        def delete_record(operation, index)
          record = find_batch_record(operation)
          authorize record, :destroy?
          record.with_lock do
            validate_precondition!(record, operation, index)
            ensure_deletable!(record, index)
            result = batch_result(record, index, 'delete')
            record.destroy!
            result
          end
        end

        def validate_precondition!(record, operation, index)
          expected = operation[:if_match].to_s
          raise PreconditionRequired, "operation #{index} if_match is required" if expected.blank?
          return if ActiveSupport::SecurityUtils.secure_compare(expected, api_etag(record))

          raise SyncConflict, "operation #{index} record has changed since it was last read"
        end

        def batch_result(record, index, action)
          {
            index: index,
            action: action,
            record_type: record.class.name,
            record_portable_id: record.portable_id
          }
        end

        def ensure_deletable!(record, index)
          return unless record.is_a?(Medication)
          return unless MedicationAdministrationHistory.exists_for?(record)

          raise BatchError, "operation #{index} delete conflicts with retained administration history"
        end

        def find_batch_record(operation)
          scope = batch_scope(operation.fetch(:resource_type))
          find_api_record(scope, operation.fetch(:id))
        rescue KeyError
          raise BatchError, 'operation resource_type and id are required'
        end

        def batch_scope(resource_type)
          case resource_type
          when 'medication'
            policy_scope(Medication)
          when 'health_event'
            policy_scope(HealthEvent)
          else
            raise BatchError, "resource_type #{resource_type} is unsupported"
          end
        end

        def permitted_attributes_for(record, attributes)
          case record
          when Medication
            attributes.to_h.slice('name', 'friendly_name', 'current_supply', 'reorder_threshold')
          when HealthEvent
            attributes.to_h.slice('title', 'notes', 'severity', 'ended_on')
          else
            {}
          end
        end
      end
    end
  end
end
