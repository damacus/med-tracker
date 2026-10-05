# frozen_string_literal: true

module AiMedication
  class Suggestion
    attr_reader :medication, :doses, :sources, :errors

    def initialize(medication: {}, doses: [], sources: [], errors: [])
      @medication = medication.deep_stringify_keys
      @doses = doses.map(&:deep_stringify_keys)
      @sources = sources.map(&:deep_stringify_keys)
      @errors = errors
    end

    def empty?
      medication.blank? && doses.blank?
    end

    def as_json(*)
      {
        medication: medication,
        doses: doses,
        sources: sources,
        errors: errors
      }
    end
  end
end
