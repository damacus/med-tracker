# frozen_string_literal: true

module Api
  module Fhir
    module R4
      class PatientsController < BaseController
        def index
          render_fhir_collection(policy_scope(Person).order(:id), :patient)
        end

        def show
          render_fhir_resource(find_api_record(policy_scope(Person), params.expect(:id)), :patient)
        end
      end
    end
  end
end
