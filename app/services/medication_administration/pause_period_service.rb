# frozen_string_literal: true

module MedicationAdministration
  class PausePeriodService
    def initialize(source:, membership:, reason:, note:, started_at:)
      @source = source
      @membership = membership
      @reason = reason
      @note = note
      @started_at = started_at
    end

    def call
      source.with_lock do
        period = open_period
        return pause_source(period) if period

        pause_source(source.medication_pause_periods.create!(period_attributes))
      end
    end

    private

    attr_reader :source, :membership, :reason, :note, :started_at

    def open_period
      source.medication_pause_periods.find_by(ended_at: nil)
    end

    def pause_source(period)
      source.update!(active: false) if source.active
      period
    end

    def period_attributes
      return legacy_period_attributes unless source.active

      {
        reason:,
        note:,
        started_at:,
        recorded_by_membership: membership,
        legacy_context: reason == MedicationPausePeriod::LEGACY_REASON
      }
    end

    def legacy_period_attributes
      {
        reason: MedicationPausePeriod::LEGACY_REASON,
        started_at: nil,
        recorded_by_membership: nil,
        legacy_context: true
      }
    end
  end
end
