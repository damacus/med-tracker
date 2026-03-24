# frozen_string_literal: true

module Api
  module V1
    class MedicationTakesController < BaseController
      def index
        render_collection(
          policy_scope(MedicationTake),
          serializer: MedicationTakeSerializer,
          includes: [{ schedule: %i[person medication] }, { person_medication: %i[person medication] }, :taken_from_location, :taken_from_medication]
        )
      end
    end
  end
end
