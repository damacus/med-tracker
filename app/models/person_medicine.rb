# frozen_string_literal: true

# PersonMedicine represents a direct association between a person and a medicine
# without requiring a formal prescription. This is useful for vitamins, supplements,
# and over-the-counter medications.
class PersonMedicine < ApplicationRecord
  include TimingRestrictions

  belongs_to :person
  belongs_to :medicine
  has_many :medication_takes, dependent: :destroy

  validates :person_id, uniqueness: { scope: :medicine_id }

  def cycle_period
    # For non-prescription medicines, we use daily cycles
    1.day
  end
end
