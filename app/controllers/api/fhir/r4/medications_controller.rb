# frozen_string_literal: true

module Api
  module Fhir
    module R4
      class MedicationsController < BaseController
        def index
          render_fhir_collection(policy_scope(Medication).order(:id), :medication)
        end

        def show
          render_fhir_resource(find_api_record(policy_scope(Medication), params.expect(:id)), :medication)
        end
      end
    end
  end
end
