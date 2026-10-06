module Api
  module Sync
    class OperationCatalog
      OPERATIONS = {
        'medication_take' => %w[create],
        'medication_dose_occurrence' => %w[create update],
        'medication_pause_period' => %w[create close],
        'medication' => %w[create update delete adjust_inventory mark_as_ordered mark_as_received remove_stock],
        'medication_dosage_option' => %w[create update],
        'person' => %w[create update],
        'health_event' => %w[create update delete],
        'location' => %w[create update delete],
        'medication_review_prompt' => %w[update],
        'schedule' => %w[create update delete pause resume],
        'person_medication' => %w[create update delete pause resume reorder]
      }.transform_values(&:freeze).freeze
      ONLINE_ONLY_RESOURCES = %w[
        account profile avatar invitation household_membership person_access_grant location_membership report
        api_session api_app_token
      ].freeze

      def self.as_json
        OPERATIONS.map { |type, actions| { resource_type: type, actions: actions } }
      end

      def self.validate!(operation)
        return if OPERATIONS.fetch(operation[:resource_type], []).include?(operation[:action])

        raise CareRecordOperation::Error.new('Operation is not supported offline', code: 'sync_operation_unsupported')
      end
    end
  end
end
