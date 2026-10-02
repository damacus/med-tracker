module Api
  module V1
    class MedicationPausePeriodSerializer
      def initialize(period)
        @period = period
      end

      def as_json(*)
        identity_data.merge(context_data).merge(actor_data).merge(time_data)
      end

      private

      attr_reader :period

      def identity_data
        source = period.schedule || period.person_medication
        { id: period.portable_id, portable_id: period.portable_id,
          source_type: period.schedule_id ? 'schedule' : 'person_medication', source_id: source.portable_id }
      end

      def context_data
        { reason: period.reason, note: period.note, legacy_context: period.legacy_context }
      end

      def actor_data
        { recorded_by_membership_id: period.recorded_by_membership_id&.to_s,
          resumed_by_membership_id: period.resumed_by_membership_id&.to_s,
          recorded_by_name: period.recorded_by_membership&.person&.name,
          resumed_by_name: period.resumed_by_membership&.person&.name }
      end

      def time_data
        { started_at: period.started_at&.iso8601, ended_at: period.ended_at&.iso8601,
          created_at: period.created_at.iso8601, updated_at: period.updated_at.iso8601 }
      end
    end
  end
end
