# frozen_string_literal: true

module MedTrackerMcp
  module Tools
    class InventoryRisksTool < BaseTool
      tool_name 'medtracker_inventory_risks'
      description 'Return visible medications that are low-stock or out-of-stock.'
      input_schema(properties: {}, required: [])

      class << self
        def call(server_context:)
          context = tool_context(server_context)
          context.with_current do
            medications = context.policy_scope(Medication)
                                 .includes(:location, :schedules, :person_medications)
                                 .order(:name, :id)
                                 .select { |medication| medication.low_stock? || medication.out_of_stock? }

            payload = {
              format: 'medtracker.mcp.inventory_risks.v1',
              medications: medications.map { |medication| Api::V1::MedicationSerializer.new(medication).as_json }
            }

            response(payload, 'Visible low-stock and out-of-stock medications.')
          end
        end
      end
    end
  end
end
