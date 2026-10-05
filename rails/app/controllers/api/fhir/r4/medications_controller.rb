# frozen_string_literal: true

module Api
  module Fhir
    module R4
      class MedicationsController < BaseController
        def index
          render_fhir_collection(policy_scope(Medication).order(:id), :medication, search: search)
        end

        def show
          render_fhir_resource(find_api_record(policy_scope(Medication), params.expect(:id)), :medication)
        end

        private

        def search
          {
            _id: search_by_portable_id,
            code: lambda { |scope, value|
              scope.where(dmd_code: value).or(scope.where(barcode: value))
            },
            form: lambda { |scope, value|
              scope.where(category: value)
            }
          }
        end
      end
    end
  end
end
