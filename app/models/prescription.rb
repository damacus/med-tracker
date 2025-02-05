class Prescription < ApplicationRecord
  belongs_to :person
  belongs_to :medicine
  has_many :take_medicines, dependent: :destroy

  validates :dosage, presence: true
  validates :frequency, presence: true
  validates :start_date, presence: true
  validate :end_date_after_start_date, if: -> { end_date.present? }

  scope :active, -> { where("end_date IS NULL OR end_date >= ?", Date.current) }
  scope :inactive, -> { where("end_date < ?", Date.current) }

  def active?
    end_date.nil? || end_date >= Date.current
  end

  def recommended_dosage
    medicine.find_recommended_dosage(person.age)
  end

  def total_ml_taken_today
    take_medicines.total_ml_today
  end

  def total_ml_taken_24h
    take_medicines.total_ml_24h
  end

  def max_daily_ml
    if recommended = recommended_dosage
      recommended.amount_ml * recommended.frequency_per_day
    else
      nil # No recommended dosage found for this age
    end
  end

  def remaining_ml_allowed
    if max = max_daily_ml
      take_medicines.remaining_ml_allowed(max)
    else
      nil # No recommended dosage found for this age
    end
  end

  private

  def end_date_after_start_date
    return if end_date.blank? || start_date.blank?

    if end_date < start_date
      errors.add(:end_date, "must be after the start date")
    end
  end
end
