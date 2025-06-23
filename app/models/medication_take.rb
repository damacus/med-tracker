class MedicationTake < ApplicationRecord
  belongs_to :prescription

  validates :taken_at, presence: true
end
