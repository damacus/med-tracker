module Api
  module V1
    class DoseOccurrenceSerializer
      def initialize(occurrence)
        @occurrence = occurrence
      end

      def as_json(*)
        identity.merge(state)
      end

      private

      attr_reader :occurrence

      def identity
        { key: occurrence.key, source_type: 'schedule', source_id: occurrence.source.id,
          source_portable_id: occurrence.source.portable_id, window_starts_on: occurrence.window_starts_on.iso8601,
          position: occurrence.position, scheduled_at: occurrence.scheduled_at&.iso8601 }
      end

      def state
        { outcome: occurrence.outcome, expected: occurrence.expected?, due: occurrence.due? }.merge(resolution)
      end

      def resolution
        record = occurrence.record
        { reason: record&.reason, note: record&.note, resolved_at: record&.resolved_at&.iso8601,
          medication_take_id: record&.medication_take_id || occurrence.legacy_take&.id,
          etag: record && Api::RecordEtag.for(record) }
      end
    end
  end
end
