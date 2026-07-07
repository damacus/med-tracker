# frozen_string_literal: true

module Api
  module V1
    class AiMedicationSuggestionsController < BaseController
      def create
        authorize Medication, :finder?
        return render_not_found unless PaidFeature.enabled?(:ai_medication_help, user: current_user)

        suggestion = AiMedication::SuggestionService.new.call(
          medication_identity: medication_identity_params.to_h.deep_symbolize_keys,
          user: current_user
        )

        render json: { data: suggestion.as_json }
      end

      private

      def medication_identity_params
        return ActionController::Parameters.new if params[:medication].blank?

        params.expect(medication: %i[name barcode dmd_code dmd_system dmd_concept_class category description])
      end
    end
  end
end
