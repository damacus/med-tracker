# frozen_string_literal: true

module OpenFda
  class MedicationReviewEvidenceRefresh
    def initialize(builder: SnapshotBuilder.new, retrieved_on: Date.current)
      @builder = builder
      @retrieved_on = retrieved_on
    end

    def call
      snapshot = builder.call
      labels = snapshot.fetch('labels')
      attributes = labels.map { |label| mapper.call(label) }
      existing = existing_records.index_by(&:source_record_id)
      changes = changes_for(existing, attributes)
      importer.call(labels: labels)

      report(snapshot, labels, changes)
    end

    private

    attr_reader :builder, :retrieved_on

    def changes_for(existing, attributes)
      changes = attributes.filter_map { |values| change_for(existing[values.fetch(:source_record_id)], values) }
      changes + missing_changes(existing, attributes)
    end

    def report(snapshot, labels, changes)
      counts = change_counts(changes)
      {
        source_last_updated: Date.iso8601(snapshot.fetch('openfda_last_updated')),
        label_count: labels.size,
        created_count: counts.fetch('created', 0),
        updated_count: counts.fetch('updated', 0),
        unchanged_count: labels.size - counts.fetch('created', 0) - counts.fetch('updated', 0),
        missing_count: counts.fetch('missing', 0),
        changes: changes
      }
    end

    def change_counts(changes)
      changes.group_by { |change| change.fetch(:type) }.transform_values(&:size)
    end

    def mapper
      @mapper ||= EvidenceAttributes.new(retrieved_on: retrieved_on)
    end

    def importer
      @importer ||= MedicationReviewEvidenceImporter.new(retrieved_on: retrieved_on, attributes: mapper)
    end

    def existing_records
      MedicationReviewEvidenceRecord.where(source_name: EvidenceAttributes::SOURCE_NAME)
    end

    def change_for(record, attributes)
      return created_change(attributes) unless record
      return if attributes_unchanged?(record, attributes)

      {
        type: 'updated',
        source_record_id: record.source_record_id,
        from_version: record.source_version,
        to_version: attributes.fetch(:source_version),
        from_effective_on: record.source_effective_on&.iso8601,
        to_effective_on: attributes.fetch(:source_effective_on).iso8601
      }
    end

    def created_change(attributes)
      {
        type: 'created',
        source_record_id: attributes.fetch(:source_record_id),
        to_version: attributes.fetch(:source_version),
        to_effective_on: attributes.fetch(:source_effective_on).iso8601
      }
    end

    def missing_changes(existing, attributes)
      live_ids = attributes.pluck(:source_record_id)
      existing.except(*live_ids).values.map do |record|
        {
          type: 'missing',
          source_record_id: record.source_record_id,
          from_version: record.source_version,
          from_effective_on: record.source_effective_on&.iso8601
        }
      end
    end

    def attributes_unchanged?(record, attributes)
      comparable_keys(record).all? { |key| record.public_send(key) == attributes.fetch(key) }
    end

    def comparable_keys(record)
      source_keys = MedicationReviewEvidenceImporter::SOURCE_ATTRIBUTE_KEYS - [:retrieved_on]
      return source_keys if record.match_status == 'reviewed_pair'

      source_keys + %i[risk_level match_confidence match_status candidate_terms pharmacologic_classes interacting_terms]
    end
  end
end
