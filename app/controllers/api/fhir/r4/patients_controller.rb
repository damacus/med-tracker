# frozen_string_literal: true

module Api
  module Fhir
    module R4
      class PatientsController < BaseController
        def index
          render_fhir_collection(policy_scope(Person).order(:id), :patient, search: search)
        end

        def show
          render_fhir_resource(find_api_record(policy_scope(Person), params.expect(:id)), :patient)
        end

        private

        def search
          {
            _id: search_by_portable_id,
            name: lambda { |scope, value|
              scope.where(Person.arel_table[:name].matches("%#{Person.sanitize_sql_like(value)}%"))
            },
            birthdate: lambda { |scope, value|
              scope.where(date_of_birth: iso8601_date(value, field: 'birthdate'))
            }
          }
        end
      end
    end
  end
end
