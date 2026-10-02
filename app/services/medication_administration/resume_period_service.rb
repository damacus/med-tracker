# frozen_string_literal: true

module MedicationAdministration
  class ResumePeriodService
    class StalePrecondition < StandardError; end

    def initialize(source:, membership:, ended_at:, period: nil, expected_etag: nil)
      @source = source
      @membership = membership
      @ended_at = ended_at
      @requested_period = period
      @expected_etag = expected_etag
    end

    def call
      source.with_lock do
        raise ActiveRecord::RecordNotFound if source.retired_at?

        period = addressed_or_open_period
        validate_precondition!(period)
        return period if period&.ended_at?
        return completed_period unless period || source.paused?

        period ||= build_legacy_period
        close_period(period)
      end
    end

    private

    attr_reader :source, :membership, :ended_at, :requested_period, :expected_etag

    def validate_precondition!(period)
      return if expected_etag.blank? || expected_etag == Api::RecordEtag.for(period)

      raise StalePrecondition
    end

    def addressed_or_open_period
      requested_period ? source.medication_pause_periods.find(requested_period.id) : open_period
    end

    def close_period(period)
      period.update!(ended_at:, resumed_by_membership: membership)
      source.update!(active: true) if source.paused?
      period
    end

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
