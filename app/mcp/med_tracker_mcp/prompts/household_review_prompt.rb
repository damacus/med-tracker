# frozen_string_literal: true

require 'mcp'

module MedTrackerMcp
  module Prompts
    class HouseholdReviewPrompt < MCP::Prompt
      prompt_name 'medtracker_household_review'
      title 'MedTracker Household Review'
      description 'Review visible medication schedules, inventory risks, and recent health history for a household.'

      class << self
        def template(_args, server_context:)
          context = ToolContext.new(server_context)
          MCP::Prompt::Result.new(
            description: 'Review MedTracker household medication context.',
            messages: [MCP::Prompt::Message.new(role: 'user', content: content(context))]
          )
        end

        private

        def content(context)
          {
            type: 'text',
            text: <<~TEXT.squish
              Review the visible MedTracker household context for #{context.household.name}. Focus on missed or taken
              doses today, low inventory, people with active medication schedules, and recent health-history patterns.
              Do not infer access beyond the authenticated household membership.
            TEXT
          }
        end
      end
    end
  end
end
