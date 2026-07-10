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
            format: ['json', FHIR_JSON],
            rest: [{ mode: 'server', security: smart_security, resource: resources }]
          }, content_type: FHIR_JSON
        end

        private

        def smart_security
          {
            service: [{
              coding: [{
                system: 'http://terminology.hl7.org/CodeSystem/restful-security-service',
                code: 'SMART-on-FHIR'
              }]
            }],
            extension: [{
              url: 'http://fhir-registry.smarthealthit.org/StructureDefinition/oauth-uris',
              extension: %w[authorize token revoke].map do |endpoint|
                { url: endpoint, valueUri: "#{request.base_url}/#{endpoint}" }
              end
            }]
          }
        end

        def resources
          [
            resource('Patient', %w[_id name birthdate]),
            resource('Medication', %w[_id code form]),
            resource('MedicationRequest', %w[_id patient subject medication status date]),
            resource('MedicationStatement', %w[_id patient subject medication status]),
            resource('MedicationAdministration', %w[_id patient subject medication status date])
          ]
        end

        def resource(type, search_parameters)
          {
            type: type,
            interaction: [{ code: 'read' }, { code: 'search-type' }],
            searchParam: search_parameters.map { |name| { name: name, type: search_parameter_type(name) } }
          }
        end

        def search_parameter_type(name)
          case name
          when '_id', 'patient', 'subject', 'medication', 'status', 'code', 'form'
            'token'
          when 'date', 'birthdate'
            'date'
          else
            'string'
          end
        end
      end
    end
  end
end
