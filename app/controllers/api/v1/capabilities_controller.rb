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
          administration: administration,
          portable_formats: portable_formats,
          backups: backups,
          fhir: fhir,
          sync: sync,
          client_tools: client_tools
        }
      end

      def authentication
        {
          methods: %w[bearer_session api_app_token],
          hosted_mobile: 'oidc_authorization_code_pkce',
          password_login: 'development_or_migration',
          oidc_exchange: {
            supported: true,
            pkce_required: true,
            session_listing: true,
            session_revocation: true
          }
        }
      end

      def administration
        {
          household: true,
          fresh_mfa_required: true,
          app_tokens: true,
          audit_logs: true,
          invitations: true,
          person_access_grants: true
        }
      end

      def portable_formats
        %w[
          medtracker.portable.v1
          medtracker.portable.encrypted.v1
          medtracker.portable.v2
        ]
      end

      def backups
        {
          encrypted_migration_bundle: true,
          unencrypted_zip: true,
          health_data_json: true
        }
      end

      def fhir
        {
          version: 'R4',
          resources: %w[
            Patient
            Medication
            MedicationRequest
            MedicationStatement
            MedicationAdministration
          ]
        }
      end

      def sync
        {
          portable_ids: true,
          numeric_ids: 'backward_compatible',
          mobile_snapshot: true,
          dry_run_import: true,
          idempotency_keys: true,
          etag_conflicts: true,
          change_feed: true,
          batch_mutations: true,
          tombstones: true
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
