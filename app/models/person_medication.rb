# frozen_string_literal: true

# PersonMedication represents a direct association between a person and a medication
# without requiring a formal schedule. This is useful for vitamins, supplements,
# and over-the-counter medications.
class PersonMedication < ApplicationRecord
  include TimingRestrictions

  belongs_to :person
  belongs_to :medication
  has_many :medication_takes, dependent: :destroy

  enum :dose_cycle, { daily: 0, weekly: 1, monthly: 2 }, prefix: :dose

  # CRITICAL: Audit trail for person-medication links
  # Tracks: which medications are assigned to which people, and their non-scheduled dosing rules
  # @see docs/audit-trail.md
  has_paper_trail

  scope :ordered, -> { order(:position, :id) }

  before_validation :assign_position, on: :create

  validates :person_id, uniqueness: { scope: :medication_id }

  def reorder(direction)
    adjacent = adjacent_record(direction)
    return false unless adjacent

    swap_positions_with(adjacent)
  end

  def cycle_period
    case dose_cycle
    when 'weekly' then 1.week
    when 'monthly' then 1.month
    else 1.day
    end
  end

  private

  def assign_position
    return if position.present?

    self.position = person.person_medications.maximum(:position).to_i + 1
  end

  def adjacent_record(direction)
    case direction
    when 'up'
      person.person_medications.where(position: ...position).order(position: :desc, id: :desc).first
    when 'down'
      person.person_medications.where(position: (position + 1)..).order(position: :asc, id: :asc).first
    end
  end

  def swap_positions_with(other)
    self_position = position

    transaction do
      update!(position: other.position)
      other.update!(position: self_position)
    end
  end
end
