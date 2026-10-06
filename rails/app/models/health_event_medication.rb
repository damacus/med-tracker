# frozen_string_literal: true

class HealthEventMedication < ApplicationRecord
  belongs_to :household
  belongs_to :health_event
  belongs_to :medication, optional: true

  before_validation :assign_household
  before_validation :snapshot_medication_name
  after_create :touch_sync_health_event
  after_update :touch_sync_health_event
  after_destroy :touch_sync_health_event

  validates :medication_name, presence: true
  validates :medication_id, uniqueness: {
    scope: :health_event_id,
    message: :linked_to_health_event
  }, allow_nil: true

  private

  def touch_sync_health_event
    health_event.refresh_sync_version! unless destroyed_by_association
  end

  def assign_household
    self.household ||= health_event&.household
  end

  def snapshot_medication_name
    self.medication_name = medication&.name if medication_name.blank? && medication.present?
  end
end
