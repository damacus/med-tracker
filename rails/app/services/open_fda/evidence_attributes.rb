# frozen_string_literal: true

module OpenFda
  class EvidenceAttributes
    SOURCE_NAME = 'openFDA / DailyMed SPL'

    def initialize(retrieved_on: Date.current)
      @retrieved_on = retrieved_on
    end

    def call(label)
      openfda = label.fetch('openfda', {})
      source_attributes(label, openfda).merge(identity_attributes(label, openfda), review_attributes(label))
    end

    private

    attr_reader :retrieved_on

    def source_attributes(label, openfda)
      source_record_id = label.fetch('set_id')
      {
        source_name: SOURCE_NAME,
        source_record_id: source_record_id,
        source_url: "https://dailymed.nlm.nih.gov/dailymed/drugInfo.cfm?setid=#{source_record_id}",
        source_version: label.fetch('version'),
        source_effective_on: Date.strptime(label.fetch('effective_time'), '%Y%m%d'),
        retrieved_on: retrieved_on,
        product_name: first_value(openfda, 'brand_name') || first_value(openfda, 'generic_name') || 'Unnamed product',
        label_section: 'Drug Interactions',
        evidence_text: Array(label.fetch('drug_interactions')).join("\n\n")
      }
    end

    def review_attributes(_label)
      {
        risk_level: 'unknown',
        match_confidence: 'unknown',
        match_status: 'unreviewed',
        interacting_terms: []
      }
    end

    def identity_attributes(label, openfda)
      substances = Array(openfda['substance_name'])
      selection_term = selection_term(label, openfda, substances)
      matching_substances = matching_substances(substances, selection_term)
      {
        active_ingredient: matching_substances.first || substances.first,
        candidate_terms: candidate_terms(openfda, substances, matching_substances, selection_term),
        pharmacologic_classes: substances.one? ? normalized_values(openfda, 'pharm_class_epc') : []
      }
    end

    def selection_term(label, openfda, substances)
      value = label['selection_term'] || substances.first || first_value(openfda, 'generic_name')
      MedicationReviewTermNormalizer.label(value)
    end

    def matching_substances(substances, selection_term)
      substances.select { |substance| MedicationReviewTermNormalizer.label(substance).include?(selection_term) }
    end

    def candidate_terms(openfda, substances, matching_substances, selection_term)
      terms = [selection_term] + matching_substances.map { |value| MedicationReviewTermNormalizer.label(value) }
      terms += normalized_values(openfda, 'generic_name') if substances.one?
      terms.compact_blank.uniq
    end

    def normalized_values(hash, *keys)
      keys.flat_map { |key| Array(hash[key]) }
          .map { |value| MedicationReviewTermNormalizer.label(value) }
          .compact_blank
          .uniq
    end

    def first_value(hash, key)
      Array(hash[key]).first.presence
    end
  end
end
