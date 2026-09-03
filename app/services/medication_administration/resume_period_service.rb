# frozen_string_literal: true

module MedicationAdministration
  class ResumePeriodService
    def initialize(source:, membership:, ended_at:)
      @source = source
      @membership = membership
      @ended_at = ended_at
    end

    def call
      source.with_lock do
        period = open_period
        return completed_period unless period || source.paused?

        period ||= build_legacy_period
        period.update!(ended_at:, resumed_by_membership: membership)
        source.update!(active: true) if source.paused?
        period
      end
    end

    private

    attr_reader :source, :membership, :ended_at

    def open_period
      source.medication_pause_periods.find_by(ended_at: nil)
    end

    def completed_period
      source.medication_pause_periods.where.not(ended_at: nil).order(ended_at: :desc, id: :desc).first
    end

    def build_legacy_period
      source.medication_pause_periods.build(
        reason: MedicationPausePeriod::LEGACY_REASON,
        started_at: nil,
        recorded_by_membership: nil,
        legacy_context: true
      )
    end
  end
end
