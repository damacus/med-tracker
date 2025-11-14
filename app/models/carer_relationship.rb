# frozen_string_literal: true

# CarerRelationship represents the relationship between a carer and a patient.
# This allows tracking who is responsible for administering medication to whom.
class CarerRelationship < ApplicationRecord
  belongs_to :carer, class_name: 'Person', inverse_of: :patient_relationships
  belongs_to :patient, class_name: 'Person', inverse_of: :carer_relationships

  # Audit trail for carer assignments and removals
  # Important for safeguarding: tracks who has access to patient records
  # Tracks: carer assignments, relationship types, activation/deactivation
  # @see docs/audit-trail.md
  has_paper_trail

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
