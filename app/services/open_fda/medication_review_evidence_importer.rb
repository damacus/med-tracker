# frozen_string_literal: true

module OpenFda
  class MedicationReviewEvidenceImporter
    SOURCE_ATTRIBUTE_KEYS = %i[
      source_name source_record_id source_url source_version source_effective_on retrieved_on product_name
      active_ingredient label_section evidence_text
    ].freeze

    def initialize(client: SnapshotClient.new, retrieved_on: Date.current, attributes: nil)
      @client = client
      @attributes = attributes || EvidenceAttributes.new(retrieved_on: retrieved_on)
    end

    def call(limit: 80, labels: nil)
      (labels || client.labels(limit: limit)).map { |label| import_label(label) }
    end

    private

    attr_reader :client, :attributes

    def import_label(label)
      imported_attributes = attributes.call(label)
      record = MedicationReviewEvidenceRecord.find_or_initialize_by(
        source_record_id: imported_attributes.fetch(:source_record_id)
      )
      record.assign_attributes(imported_attributes.slice(*SOURCE_ATTRIBUTE_KEYS))
      unless record.persisted? && record.match_status == 'reviewed_pair'
        record.assign_attributes(imported_attributes.except(*SOURCE_ATTRIBUTE_KEYS))
      end
      record.save!
      record
    end
  end
end
