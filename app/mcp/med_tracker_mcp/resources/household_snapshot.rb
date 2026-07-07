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
          payload = Tools::HouseholdSnapshotTool.call(server_context: server_context).to_h.fetch(:structuredContent)

          MCP::Resource::TextContents.new(
            uri: URI,
            mime_type: 'application/json',
            text: JSON.generate(payload)
          )
        end
      end
    end
  end
end
