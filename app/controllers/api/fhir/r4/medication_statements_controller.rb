# frozen_string_literal: true

module Api
  module Fhir
    module R4
      class MedicationStatementsController < BaseController
        def index
          scope = policy_scope(PersonMedication).includes(:person, :medication).order(:id)
          render_fhir_collection(scope, :medication_statement)
        end

        def show
          person_medication = find_api_record(policy_scope(PersonMedication).includes(:person, :medication),
                                              params.expect(:id))
          render_fhir_resource(person_medication, :medication_statement)
        end
      end
    end
  end
end
