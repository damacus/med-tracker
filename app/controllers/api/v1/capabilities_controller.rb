# frozen_string_literal: true

module Api
  module V1
    class CapabilitiesController < ActionController::API
      def show
        response.set_header('Cache-Control', 'no-store')

        render json: { data: capabilities }
      end

      private

      def capabilities
        {
          format: 'medtracker.api.capabilities.v1',
          api_version: 'v1',
          authentication: authentication,
          portable_formats: portable_formats,
          sync: sync,
          client_tools: client_tools
        }
      end

      def authentication
        {
          methods: %w[bearer_session api_app_token],
          hosted_mobile: 'oidc_authorization_code_pkce',
          password_login: 'development_or_migration'
        }
      end

      def portable_formats
        %w[
          medtracker.portable.v1
          medtracker.portable.encrypted.v1
        ]
      end

      def sync
        {
          portable_ids: true,
          numeric_ids: 'backward_compatible',
          mobile_snapshot: true,
          dry_run_import: true
        }
      end

      def client_tools
        {
          cli: deferred_client_tool,
          mcp_server: {
            supported: true,
            transport: 'streamable_http',
            endpoint: '/mcp',
            tools: MedTrackerMcp::Server::TOOLS.map(&:name_value),
            resources: MedTrackerMcp::Server::RESOURCES.map { |resource| resource::URI }
          },
          diagnostics: %w[request_id retry_after]
        }
      end

      def deferred_client_tool
        {
          supported: false,
          status: 'deferred'
        }
      end
    end
  end
end
