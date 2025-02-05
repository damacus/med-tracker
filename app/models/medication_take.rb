class MedicationTake < ApplicationRecord
  belongs_to :prescription

  validates :taken_at, presence: true
  validate :taken_at_not_in_future

  # Scopes
  scope :today, -> { where(taken_at: Time.current.beginning_of_day..Time.current.end_of_day) }
  scope :this_week, -> { where(taken_at: Time.current.beginning_of_week..Time.current.end_of_week) }
  scope :recent, -> { order(taken_at: :desc) }

  # Delegate methods to access prescription details easily
  delegate :person, :medicine, to: :prescription

  private

  def taken_at_not_in_future
    if taken_at.present? && taken_at > Time.current
      errors.add(:taken_at, "cannot be in the future")
    end
  end
end
