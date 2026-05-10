# frozen_string_literal: true

module AiMedication
  class RubyLlmAssistant
    MODEL_ENV = "MEDTRACKER_AI_MEDICATION_HELP_MODEL"

    def call(medication_identity:)
      return Suggestion.new(errors: ["ruby_llm_unavailable"]) unless ruby_llm_available?
      return Suggestion.new(errors: ["ruby_llm_unconfigured"]) unless configured?

      response = chat.ask(prompt_for(medication_identity))
      suggestion_from(response.content)
    end

    private

    def ruby_llm_available?
      defined?(RubyLLM)
    end

    def configured?
      %w[
        OPENAI_API_KEY
        ANTHROPIC_API_KEY
        GEMINI_API_KEY
        AZURE_API_KEY
        OPENROUTER_API_KEY
      ].any? { |key| ENV.fetch(key, nil).present? }
    end

    def chat
      model = ENV.fetch(MODEL_ENV, nil).presence
      base = (model ? RubyLLM.chat(model: model) : RubyLLM.chat).with_instructions(instructions)
      return base unless base.respond_to?(:with_tools)

      base.with_tools(
        Tools::SearchMedicationSources.new,
        Tools::FetchMedicationSource.new,
        Tools::ExtractMedicationGuidance.new
      )
    end

    def instructions
      "Help fill medication onboarding fields only from trusted source tool results. " \
        "Return JSON with medication, doses, and sources. Do not guess missing dose guidance."
    end

    def prompt_for(medication_identity)
      {
        task: "Find trusted source evidence and draft medication onboarding fields.",
        medication_identity: medication_identity
      }.to_json
    end

    def suggestion_from(content)
      payload = content.is_a?(Hash) ? content : JSON.parse(content.to_s)
      Suggestion.new(
        medication: payload.fetch("medication", {}),
        doses: payload.fetch("doses", []),
        sources: payload.fetch("sources", []),
        errors: payload.fetch("errors", [])
      )
    rescue JSON::ParserError
      Suggestion.new(errors: ["invalid_model_response"])
    end
  end
end
