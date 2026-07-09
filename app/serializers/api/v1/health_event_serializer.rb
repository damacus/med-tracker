# frozen_string_literal: true

module Api
  module V1
    class HealthEventSerializer
      def initialize(health_event)
        @health_event = health_event
      end

      def as_json(*)
        identity_data.merge(event_data).merge(date_data).merge(medication_data)
      end

      private

      attr_reader :health_event

      def identity_data
        {
          id: health_event.id,
          portable_id: health_event.portable_id,
          person_id: health_event.person_id,
          person_portable_id: health_event.person&.portable_id
        }
      end

      def event_data
        {
          event_kind: health_event.event_kind,
          severity: health_event.severity,
          title: health_event.title,
          notes: health_event.notes
        }
      end

      def date_data
        {
          started_on: health_event.started_on&.iso8601,
          ended_on: health_event.ended_on&.iso8601,
          updated_at: health_event.updated_at.iso8601
        }
      end

      def medication_data
        {
          medication_ids: health_event.medication_ids,
          medication_portable_ids: health_event.medications.map(&:portable_id)
        }
      end
    end
  end
end
