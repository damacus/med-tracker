# frozen_string_literal: true

module Api
  module Fhir
    module R4
      class MetadataController < BaseController
        def show
          render json: {
            resourceType: 'CapabilityStatement',
            status: 'active',
            date: Time.current.iso8601,
            kind: 'instance',
            fhirVersion: '4.0.1',
            format: ['json'],
            rest: [{ mode: 'server', resource: resources }]
          }
        end

        private

        def resources
          %w[Patient Medication MedicationRequest MedicationStatement MedicationAdministration].map do |type|
            { type: type, interaction: [{ code: 'read' }, { code: 'search-type' }] }
          end
        end
      end
    end
  end
end
