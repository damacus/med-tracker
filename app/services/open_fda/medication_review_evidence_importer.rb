# frozen_string_literal: true

module OpenFda
  class MedicationReviewEvidenceImporter
    SOURCE_NAME = 'openFDA / DailyMed SPL'

    def initialize(client: DrugLabelClient.new, retrieved_on: Date.current)
      @client = client
      @retrieved_on = retrieved_on
    end

    def call(limit: 80)
      client.labels(limit: limit).map { |label| import_label(label) }
    end

    private

    attr_reader :client, :retrieved_on

    def import_label(label)
      source_record_id = label.fetch('set_id')
      record = MedicationReviewEvidenceRecord.find_or_initialize_by(source_record_id: source_record_id)
      record.assign_attributes(source_attributes(label, source_record_id))
      unless record.persisted? && record.match_status == 'reviewed_pair'
        record.assign_attributes(automatic_attributes(label))
      end
      record.save!
      record
    end

    def source_attributes(label, source_record_id)
      openfda = label.fetch('openfda', {})
      {
        source_name: SOURCE_NAME,
        source_url: "https://dailymed.nlm.nih.gov/dailymed/drugInfo.cfm?setid=#{source_record_id}",
        retrieved_on: retrieved_on,
        product_name: first_value(openfda, 'brand_name') || first_value(openfda, 'generic_name') || 'Unnamed product',
        active_ingredient: first_value(openfda, 'substance_name'),
        label_section: 'Drug Interactions',
        evidence_text: Array(label.fetch('drug_interactions')).join("\n\n")
      }
    end

    def automatic_attributes(label)
      openfda = label.fetch('openfda', {})
      {
        risk_level: 'unknown',
        match_confidence: 'unknown',
        match_status: 'unreviewed',
        candidate_terms: normalized_values(openfda, 'substance_name', 'generic_name'),
        pharmacologic_classes: normalized_values(openfda, 'pharm_class_epc'),
        interacting_terms: []
      }
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
