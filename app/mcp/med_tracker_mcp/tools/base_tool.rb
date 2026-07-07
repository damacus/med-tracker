# frozen_string_literal: true

require 'mcp'

module MedTrackerMcp
  module Tools
    class BaseTool < MCP::Tool
      class << self
        private

        def tool_context(server_context)
          ToolContext.new(server_context)
        end

        def response(payload, summary)
          MCP::Tool::Response.new(
            [{ type: 'text', text: summary }],
            structured_content: payload
          )
        end

        def error_response(message)
          MCP::Tool::Response.new(
            [{ type: 'text', text: message }],
            error: true
          )
        end
      end
    end
  end
end
