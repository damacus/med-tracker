# frozen_string_literal: true

module Api
  module V1
    module Sync
      class BatchesController < Api::V1::BaseController
        class BatchError < StandardError; end

        def create
          results = []
          ActiveRecord::Base.transaction(requires_new: true) do
            operations.each_with_index do |operation, index|
              results << apply_operation(operation, index)
            end
          end

          render json: { data: { applied: true, results: results } }, status: :created
        rescue BatchError => e
          render_unprocessable(e.message)
        end

        private

        def operations
          params.expect(batch: [{ operations: [[:action, :resource_type, :id, { attributes: {} }]] }])
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
          attributes = permitted_attributes_for(record, operation.fetch(:attributes, {}))
          raise BatchError, "operation #{index} attributes are invalid" unless record.update(attributes)

          record_api_change(record, action: 'update')
          { index: index, action: 'update', record_type: record.class.name, record_portable_id: record.portable_id }
        end

        def delete_record(operation, index)
          record = find_batch_record(operation)
          authorize record, :destroy?
          ensure_deletable!(record, index)
          portable_id = record.portable_id
          record_type = record.class.name
          record.destroy!
          ApiTombstone.create!(
            household: current_household,
            account: current_account,
            household_membership: current_membership,
            record_type: record_type,
            record_portable_id: portable_id,
            action: 'delete',
            deleted_at: Time.current,
            metadata: { record_type: record_type }
          )
          { index: index, action: 'delete', record_type: record_type, record_portable_id: portable_id }
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
