# frozen_string_literal: true

module Api
  module Fhir
    module R4
      class MedicationRequestsController < BaseController
        def index
          scope = policy_scope(Schedule).includes(:person, :medication).order(:id)
          render_fhir_collection(scope, :medication_request, search: search)
        end

        def show
          schedule = find_api_record(policy_scope(Schedule).includes(:person, :medication), params.expect(:id))
          render_fhir_resource(schedule, :medication_request)
        end

        private

        def search
          {
            _id: search_by_portable_id,
            patient: patient_search,
            subject: patient_search,
            medication: medication_search,
            status: status_search,
            date: lambda { |scope, value|
              scope.where(start_date: iso8601_date(value, field: 'date'))
            }
          }
        end

        def patient_search
          lambda do |scope, value|
            person = Person.find_by!(portable_id: portable_reference_id(value), household: current_household)
            scope.where(person_id: person.id)
          end
        end

        def medication_search
          lambda do |scope, value|
            medication = Medication.find_by!(portable_id: portable_reference_id(value), household: current_household)
            scope.where(medication_id: medication.id)
          end
        end

        def status_search
          lambda do |scope, value|
            case value.to_s
            when 'active'
              scope.where(active: true)
            when 'stopped'
              scope.where(active: false)
            else
              raise InvalidFilterValue, 'status must be active or stopped'
            end
          end
        end
      end
    end
  end
end
