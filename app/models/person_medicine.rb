# frozen_string_literal: true

# PersonMedicine represents a direct association between a person and a medicine
# without requiring a formal prescription. This is useful for vitamins, supplements,
# and over-the-counter medications.
class PersonMedicine < ApplicationRecord
  include TimingRestrictions

  belongs_to :person
  belongs_to :medicine
  has_many :medication_takes, dependent: :destroy

  scope :ordered, -> { order(:position, :id) }

  before_validation :assign_position, on: :create

  validates :person_id, uniqueness: { scope: :medicine_id }

  def reorder(direction)
    adjacent = adjacent_record(direction)
    return false unless adjacent

    swap_positions_with(adjacent)
  end

  def cycle_period
    # For non-prescription medicines, we use daily cycles
    1.day
  end

  private

  def assign_position
    return if position.present?

    self.position = person.person_medicines.maximum(:position).to_i + 1
  end

  def adjacent_record(direction)
    case direction
    when 'up'
      person.person_medicines.where(position: ...position).order(position: :desc, id: :desc).first
    when 'down'
      person.person_medicines.where(position: (position + 1)..).order(position: :asc, id: :asc).first
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
