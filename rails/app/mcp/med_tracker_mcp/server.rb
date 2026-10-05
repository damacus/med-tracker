# frozen_string_literal: true

require 'mcp'

module MedTrackerMcp
  class Server
    TOOLS = [
      Tools::CurrentUserTool,
      Tools::HouseholdSnapshotTool,
      Tools::TodayScheduleTool,
      Tools::InventoryRisksTool,
      Tools::HealthHistorySummaryTool
    ].freeze

    RESOURCES = [
      Resources::HouseholdSnapshot
    ].freeze

    PROMPTS = [
      Prompts::HouseholdReviewPrompt
    ].freeze

    class << self
      def build(server_context:)
        MCP::Server.new(
          name: 'med_tracker',
          title: 'MedTracker MCP',
          version: version,
          description: 'Authenticated read-only medication context for MedTracker households.',
          instructions: instructions,
          tools: TOOLS,
          prompts: PROMPTS,
          resources: RESOURCES.map(&:definition),
          server_context: server_context,
          capabilities: capabilities
        ).tap do |server|
          define_resource_reader(server)
        end
      end

      private

      def define_resource_reader(server)
        server.resources_read_handler do |params, server_context:|
          resource = RESOURCES.find { |candidate| params.fetch(:uri) == candidate::URI }
          raise MCP::Server::ResourceNotFoundError.new(params.fetch(:uri), params) unless resource

          [resource.read(server_context: server_context).to_h]
        end
      end

      def version
        Rails.application.config.respond_to?(:x) ? Rails.application.config.x.app_version.presence || '0.1.0' : '0.1.0'
      end

      def instructions
        'Use these tools for read-only medication context. Never request or expose raw authentication tokens.'
      end

      def capabilities
        {
          tools: { listChanged: false },
          prompts: { listChanged: false },
          resources: { listChanged: false }
        }
      end
    end
  end
end
