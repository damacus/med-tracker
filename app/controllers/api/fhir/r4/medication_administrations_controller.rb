# frozen_string_literal: true

module Api
  module Fhir
    module R4
      class MedicationAdministrationsController < BaseController
        def index
          scope = policy_scope(MedicationTake)
                  .includes({ schedule: %i[person medication] }, { person_medication: %i[person medication] },
                            :taken_from_medication)
                  .order(:id)
          render_fhir_collection(scope, :medication_administration)
        end

        def show
          take = find_api_record(
            policy_scope(MedicationTake).includes({ schedule: %i[person medication] },
                                                  { person_medication: %i[person medication] },
                                                  :taken_from_medication),
            params.expect(:id)
          )
          render_fhir_resource(take, :medication_administration)
        end
      end
    end
  end
end
