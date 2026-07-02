# frozen_string_literal: true

class HealthEvent < ApplicationRecord
  has_paper_trail

  belongs_to :household
  belongs_to :person
  has_many :health_event_medications, dependent: :destroy
  has_many :medications, through: :health_event_medications

  enum :event_kind, { illness: 0, suspected_side_effect: 1 }
  enum :severity, { mild: 0, moderate: 1, severe: 2 }

  before_validation :assign_household

  validates :event_kind, :title, :started_on, presence: true
  validate :ended_on_not_before_started_on

  def ongoing? = ended_on.nil?

  private

  def assign_household
    self.household ||= person&.household
  end

  def ended_on_not_before_started_on
    return if ended_on.blank? || started_on.blank? || ended_on >= started_on

    errors.add(:ended_on, :on_or_after_start_date)
  end
end
