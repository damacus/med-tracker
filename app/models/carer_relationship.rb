# frozen_string_literal: true

# CarerRelationship represents the relationship between a carer and a patient.
# This allows tracking who is responsible for administering medication to whom.
class CarerRelationship < ApplicationRecord
  belongs_to :carer, class_name: 'Person'
  belongs_to :patient, class_name: 'Person'

  validates :carer_id, uniqueness: { scope: :patient_id }
  validates :relationship_type, presence: true

  scope :active, -> { where(active: true) }

  def deactivate!
    update!(active: false)
  end

  def activate!
    update!(active: true)
  end
end
