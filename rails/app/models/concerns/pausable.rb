# frozen_string_literal: true

module Pausable
  def paused? = !active

  def pause!(reason: MedicationPausePeriod::LEGACY_REASON, note: nil, membership: Current.membership,
             started_at: Time.current)
    MedicationAdministration::PausePeriodService.new(source: self, membership:, reason:, note:, started_at:).call
    self
  end

  def resume!(membership: Current.membership, ended_at: Time.current)
    MedicationAdministration::ResumePeriodService.new(source: self, membership:, ended_at:).call
    self
  end
end
