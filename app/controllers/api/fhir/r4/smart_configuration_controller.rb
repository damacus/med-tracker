# frozen_string_literal: true

module Api
  module Fhir
    module R4
      class SmartConfigurationController < ActionController::API
        def show
          render json: {
            authorization_endpoint: endpoint('/authorize'),
            token_endpoint: endpoint('/token'),
            revocation_endpoint: endpoint('/revoke'),
            capabilities: %w[launch-standalone client-public permission-v2],
            code_challenge_methods_supported: ['S256'],
            grant_types_supported: %w[authorization_code refresh_token],
            response_types_supported: ['code'],
            token_endpoint_auth_methods_supported: %w[none client_secret_basic client_secret_post],
            scopes_supported: scopes_supported
          }
        end

        private

        def endpoint(path)
          "#{request.base_url}#{path}"
        end

        def scopes_supported
          %w[
            launch/patient
            offline_access
            online_access
            patient/*.rs
            patient/Patient.rs
            patient/Medication.rs
            patient/MedicationRequest.rs
            patient/MedicationStatement.rs
            patient/MedicationAdministration.rs
            user/*.rs
          ]
        end
      end
    end
  end
end
