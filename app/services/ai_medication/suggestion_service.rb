# frozen_string_literal: true

module AiMedication
  class SuggestionService
    def initialize(assistant: RubyLlmAssistant.new, validator: SuggestionValidator.new, audit_logger: AuditLogger.new)
      @assistant = assistant
      @validator = validator
      @audit_logger = audit_logger
    end

    def call(medication_identity:, user:)
      raw_suggestion = @assistant.call(medication_identity: medication_identity)
      suggestion = @validator.call(raw_suggestion)
      record_audit(user: user, medication_identity: medication_identity, suggestion: suggestion)
      suggestion
    rescue StandardError => e
      Rails.logger.warn("AI medication suggestion failed: #{e.class}: #{e.message}")
      error_suggestion = Suggestion.new(errors: ['suggestion_unavailable'])
      record_audit(user: user, medication_identity: medication_identity, suggestion: error_suggestion)
      error_suggestion
    end

    private

    def record_audit(user:, medication_identity:, suggestion:)
      @audit_logger.record(user: user, medication_identity: medication_identity, suggestion: suggestion)
    rescue StandardError => e
      Rails.logger.warn("AI medication audit logging failed: #{e.class}: #{e.message}")
    end
  end
end
