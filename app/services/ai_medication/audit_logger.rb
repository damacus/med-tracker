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
        item_id: 0,
        event: 'ai_medication/suggestion',
        object: version_object(medication_identity:, suggestion:).to_json,
        whodunnit: user&.id&.to_s,
        ip: PaperTrail.request.controller_info&.dig(:ip),
        request_id: PaperTrail.request.controller_info&.dig(:request_id),
        household_id: household_id,
        actor_membership_id: actor_membership_id,
        created_at: Time.current
      }
    end

    def version_object(medication_identity:, suggestion:)
      {
        identity_hash: identity_hash(medication_identity),
        source_count: suggestion.sources.size,
        dose_count: suggestion.doses.size,
        error_count: suggestion.errors.size,
        result_status: result_status(suggestion)
      }
    end

    def identity_hash(medication_identity)
      Digest::SHA256.hexdigest(medication_identity.to_s.strip.downcase)
    end

    def result_status(suggestion)
      suggestion.errors.any? ? 'error' : 'found'
    end

    def household_id
      PaperTrail.request.controller_info&.dig(:household_id) || Current.household&.id
    end

    def actor_membership_id
      PaperTrail.request.controller_info&.dig(:actor_membership_id) || Current.membership&.id
    end
  end
end
