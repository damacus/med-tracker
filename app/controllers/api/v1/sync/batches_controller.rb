# frozen_string_literal: true

module Api
  module V1
    module Sync
      class BatchesController < Api::V1::BaseController
        class BatchError < StandardError; end
        class PreconditionRequired < BatchError; end
        class SyncConflict < BatchError; end

        def create
          results = []
          ActiveRecord::Base.transaction(requires_new: true) do
            operations.each_with_index do |operation, index|
              results << apply_operation(operation, index)
            end
          end

          render json: { data: { applied: true, results: results } }, status: :created
        rescue PreconditionRequired => e
          render_api_error(code: 'precondition_required', message: e.message, status: :precondition_required)
        rescue SyncConflict => e
          render_conflict(e.message, code: 'sync_conflict')
        rescue BatchError => e
          render_unprocessable(e.message)
        end

        private

        def operations
          params.expect(batch: [{ operations: [[:action, :resource_type, :id, :if_match, { attributes: {} }]] }])
                .fetch(:operations)
        end

        def apply_operation(operation, index)
          case operation.fetch(:action)
          when 'update'
            update_record(operation, index)
          when 'delete'
            delete_record(operation, index)
          else
            raise BatchError, "operation #{index} action is unsupported"
          end
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
