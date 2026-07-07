# frozen_string_literal: true

module MedTrackerMcp
  module Tools
    class HouseholdSnapshotTool < BaseTool
      tool_name 'medtracker_household_snapshot'
      description 'Return a policy-scoped MedTracker household snapshot for the authenticated membership.'
      input_schema(properties: {}, required: [])

      class << self
        def call(server_context:)
          context = tool_context(server_context)
          context.with_current do
            snapshot = PortableData::Exporter.new(
              household: context.household,
              membership: context.membership,
              passphrase: nil,
              request: nil
            ).mobile_snapshot

            payload = {
              format: 'medtracker.mcp.household_snapshot.v1',
              snapshot: snapshot
            }

            response(payload, 'Policy-scoped MedTracker household snapshot.')
          end
        end
      end
    end
  end
end
