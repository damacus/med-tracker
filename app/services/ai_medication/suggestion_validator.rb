# frozen_string_literal: true

module AiMedication
  class SuggestionValidator
    def call(suggestion)
      Suggestion.new(
        medication: valid_medication_attributes(suggestion.medication),
        doses: valid_doses(suggestion.doses),
        sources: valid_sources(suggestion.sources),
        errors: suggestion.errors
      )
    end

    private

    def valid_medication_attributes(attributes)
      attributes.slice('name', 'category', 'description', 'warnings')
    end

    def valid_doses(doses)
      doses.select { |dose| valid_dose?(dose) }
    end

    def valid_sources(sources)
      sources.select { |source| source['url'].present? && source['title'].present? }
    end

    def valid_dose?(dose)
      dose_evidence_valid?(dose['evidence']) &&
        positive_number?(dose['amount']) &&
        Medication::DOSAGE_UNITS.include?(dose['unit'].to_s) &&
        positive_integer?(dose['default_max_daily_doses']) &&
        non_negative_number?(dose['default_min_hours_between_doses']) &&
        MedicationDosageOption.default_dose_cycles.key?(dose['default_dose_cycle'].to_s)
    end

    def dose_evidence_valid?(evidence)
      evidence.is_a?(Hash) &&
        evidence['url'].present? &&
        evidence['title'].present? &&
        evidence['text'].present?
    end

    def positive_number?(value)
      BigDecimal(value.to_s).positive?
    rescue ArgumentError
      false
    end

    def non_negative_number?(value)
      BigDecimal(value.to_s) >= 0
    rescue ArgumentError
      false
    end

    def positive_integer?(value)
      value.to_s.match?(/\A\d+\z/) && value.to_i.positive?
    end
  end
end
