class TakeMedicine < ApplicationRecord
  self.table_name = "medication_takes"

  belongs_to :prescription

  validates :taken_at, presence: true
  validates :amount_ml, presence: true, numericality: { greater_than: 0 }
  validate :taken_at_not_in_future

  # Scopes
  scope :today, -> { where(taken_at: Time.current.beginning_of_day..Time.current.end_of_day) }
  scope :this_week, -> { where(taken_at: Time.current.beginning_of_week..Time.current.end_of_week) }
  scope :recent, -> { order(taken_at: :desc) }

  # Delegate methods to access prescription details easily
  delegate :person, :medicine, to: :prescription

  def self.total_ml_today
    today.sum(:amount_ml)
  end

  def self.total_ml_24h
    where(taken_at: 24.hours.ago..Time.current).sum(:amount_ml)
  end

  def self.remaining_ml_allowed(max_daily_ml)
    max_daily_ml - total_ml_24h
  end

  private

  def taken_at_not_in_future
    if taken_at.present? && taken_at > Time.current
      errors.add(:taken_at, "cannot be in the future")
    end
  end
end
