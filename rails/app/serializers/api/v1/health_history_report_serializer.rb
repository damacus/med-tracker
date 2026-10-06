module Api
  module V1
    class HealthHistoryReportSerializer
      def initialize(result, start_date:, end_date:, generated_at:)
        @result = result
        @start_date = start_date
        @end_date = end_date
        @generated_at = generated_at
      end

      def as_json(*)
        { person: person_identity, start_date: @start_date.iso8601, end_date: @end_date.iso8601,
          generated_at: @generated_at.iso8601, current_medicines: current_medicines,
          chronology: @result.chronology.map { |entry| health_event(entry) },
          medication_takes: @result.medication_takes.map { |take| medication_take(take) } }
      end

      private

      def person_identity
        { id: @result.person.id.to_s, name: @result.person.name,
          date_of_birth: @result.person.date_of_birth&.iso8601 }
      end

      def current_medicines
        @result.current_medicines.map { |medicine| { id: medicine.id.to_s, name: medicine.display_name } }
      end

      def health_event(entry)
        { id: entry.event.id.to_s, event_kind: entry.event_kind, title: entry.title,
          started_on: entry.started_on.iso8601, ended_on: entry.ended_on&.iso8601,
          ongoing: entry.ongoing?, duration_days: entry.duration_days, severity: entry.severity,
          notes: entry.notes, action_taken: entry.action_taken, medical_help_sought: entry.medical_help_sought,
          medication_names: entry.medication_names }
      end

      def medication_take(take)
        { taken_at: take.taken_at.iso8601, medication_name: take.medication_name,
          dose_amount: take.dose_amount&.to_s('F'), dose_unit: take.dose_unit,
          source_type: take.source_type.to_s, location_name: take.location_name }
      end
    end
  end
end
