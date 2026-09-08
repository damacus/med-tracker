module PortableData
  class DoseOccurrenceSerializer
    def initialize(record)
      @record = record
    end

    def as_json
      version.merge(identity).merge(outcome)
    end

    private

    attr_reader :record

    def identity
      { source_type: record.schedule_id ? 'schedule' : 'person_medication',
        source_portable_id: record.source.portable_id, window_starts_on: record.window_starts_on.iso8601,
        position: record.position, scheduled_at: record.scheduled_at&.iso8601(6) }
    end

    def version
      { portable_id: record.portable_id, updated_at: record.updated_at.iso8601, etag: Api::RecordEtag.for(record) }
    end

    def outcome
      { outcome: record.outcome, reason: record.reason, note: record.note,
        resolved_at: record.resolved_at&.iso8601(6), medication_take_portable_id: record.medication_take&.portable_id }
    end
  end
end
