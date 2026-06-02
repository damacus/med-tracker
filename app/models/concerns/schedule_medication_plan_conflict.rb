# frozen_string_literal: true

module ScheduleMedicationPlanConflict
  extend ActiveSupport::Concern

  included do
    validate :medication_plan_does_not_conflict
  end

  private

  def medication_plan_does_not_conflict
    return unless enabled_date_range?
    return if unchanged_enabled_medication_plan?
    return unless conflicting_schedule?

    errors.add(:medication, 'already has an enabled plan for this person')
  end

  def enabled_date_range?
    active && person_id.present? && medication_id.present? && start_date.present? && end_date.present?
  end

  def unchanged_enabled_medication_plan?
    persisted? && !will_save_change_to_person_id? && !will_save_change_to_medication_id? &&
      !will_save_change_to_active? && !will_save_change_to_start_date? && !will_save_change_to_end_date?
  end

  def conflicting_schedule?
    Schedule
      .where(active: true, person_id: person_id, medication_id: medication_id)
      .where.not(id: id)
      .exists?(['start_date <= ? AND end_date >= ?', end_date, start_date])
  end
end
