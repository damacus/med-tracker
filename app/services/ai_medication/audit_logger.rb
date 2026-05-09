# frozen_string_literal: true

module AiMedication
  class AuditLogger
    ITEM_TYPE = 'AiMedicationSuggestion'

    def record(user:, medication_identity:, suggestion:)
      # rubocop:disable Rails/SkipsModelValidations
      PaperTrail::Version.insert(version_attrs(user:, medication_identity:, suggestion:))
      # rubocop:enable Rails/SkipsModelValidations
    rescue StandardError => e
      Rails.logger.error("AiMedication::AuditLogger failed: #{e.class}: #{e.message}")
    end

    private

    def version_attrs(user:, medication_identity:, suggestion:)
      {
        item_type: ITEM_TYPE,
        item_id:   0,
        event:     'ai_medication/suggestion',
        object:    {
          identity_hash: Digest::SHA256.hexdigest(medication_identity.to_s.strip.downcase),
          source_count:  suggestion.sources.size,
          dose_count:    suggestion.doses.size,
          error_count:   suggestion.errors.size,
          result_status: suggestion.errors.any? ? 'error' : 'found'
        }.to_json,
        whodunnit:  user&.id&.to_s,
        ip:         PaperTrail.request.controller_info&.dig(:ip),
        request_id: PaperTrail.request.controller_info&.dig(:request_id),
        created_at: Time.current
      }
    end
  end
end
