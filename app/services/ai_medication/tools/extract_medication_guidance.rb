# frozen_string_literal: true

module AiMedication
  module Tools
    class ExtractMedicationGuidance < (defined?(RubyLLM::Tool) ? RubyLLM::Tool : Object)
      if respond_to?(:description)
        description "Accepts source-linked structured medication guidance extracted by the model"
      end

      if respond_to?(:params)
        params do
          object :suggestion, description: "Source-linked medication suggestion payload"
        end
      end

      def execute(suggestion:)
        SuggestionValidator
          .new
          .call(
            Suggestion.new(
              medication: suggestion.fetch("medication", {}),
              doses: suggestion.fetch("doses", []),
              sources: suggestion.fetch("sources", [])
            )
          )
          .as_json
      rescue StandardError => e
        {error: "invalid_suggestion", message: e.message}
      end
    end
  end
end
