# frozen_string_literal: true

module Api
  module Fhir
    module R4
      class MedicationRequestsController < BaseController
        def index
          render_fhir_collection(policy_scope(Schedule).includes(:person, :medication).order(:id), :medication_request)
        end

        def show
          schedule = find_api_record(policy_scope(Schedule).includes(:person, :medication), params.expect(:id))
          render_fhir_resource(schedule, :medication_request)
        end
      end
    end
  end
end
