# frozen_string_literal: true

module Api
  module V1
    class PersonMedicationsController < BaseController
      def index
        authorize PersonMedication
        render_collection(policy_scope(PersonMedication), serializer: PersonMedicationSerializer, includes: %i[person medication])
      end

      def show
        person_medication = policy_scope(PersonMedication).includes(:person, :medication).find(params[:id])
        authorize person_medication

        render_resource(person_medication, serializer: PersonMedicationSerializer)
      end
    end
  end
end
