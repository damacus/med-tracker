# frozen_string_literal: true

module AiMedication
  class AuditLogger
    def record(user:, medication_identity:, suggestion:)
      Rails.logger.info(
        {
          event: 'ai_medication_suggestion',
          user_id: user&.id,
          medication_identity_keys: medication_identity.keys,
          source_count: suggestion.sources.size,
          dose_count: suggestion.doses.size,
          error_count: suggestion.errors.size
        }.to_json
      )
    end
  end
end
