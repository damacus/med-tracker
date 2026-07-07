# frozen_string_literal: true

require 'mcp'

module MedTrackerMcp
  module Resources
    class HouseholdSnapshot
      URI = 'medtracker://household/snapshot'

      class << self
        def definition
          MCP::Resource.new(
            uri: URI,
            name: 'household_snapshot',
            title: 'Household Snapshot',
            description: 'Policy-scoped MedTracker household snapshot for the authenticated membership.',
            mime_type: 'application/json'
          )
        end

        def read(server_context:)
          Tools::HouseholdSnapshotTool.call(server_context: server_context).structured_content
        end
      end
    end
  end
end
