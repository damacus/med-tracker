# frozen_string_literal: true

module MedTrackerMcp
  module Tools
    class CurrentUserTool < BaseTool
      tool_name 'medtracker_current_user'
      description 'Return the authenticated MedTracker API user profile.'
      input_schema(properties: {}, required: [])

      class << self
        def call(server_context:)
          context = tool_context(server_context)
          context.with_current do
            payload = {
              format: 'medtracker.mcp.current_user.v1',
              user: Api::V1::MeSerializer.new(context.account.person.user).as_json
            }

            response(payload, 'Authenticated MedTracker user profile.')
          end
        end
      end
    end
  end
end
