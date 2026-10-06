# frozen_string_literal: true

# CarerRelationship represents the relationship between a carer and a patient.
# This allows tracking who is responsible for administering medication to whom.
class CarerRelationship < ApplicationRecord
  RELATIONSHIP_TYPES = %w[parent family_member professional_carer self].map { |v| [v.humanize, v] }.freeze

  belongs_to :household
  belongs_to :carer, class_name: 'Person', inverse_of: :patient_relationships
  belongs_to :patient, class_name: 'Person', inverse_of: :carer_relationships
  has_many :person_access_grants, dependent: :destroy

  # Audit trail for carer assignments and removals
  # Important for safeguarding: tracks who has access to patient records
  # Tracks: carer assignments, relationship types, activation/deactivation
  # @see docs/audit-trail.md
  has_paper_trail

  before_validation :assign_household_from_patient

  validates :carer_id, uniqueness: { scope: %i[patient_id household_id] }
  validates :relationship_type, presence: true
  validate :endpoints_belong_to_household

  scope :active, -> { where(active: true) }

  private

  def assign_household_from_patient
    self.household ||= patient&.household
  end

  def endpoints_belong_to_household
    return if household.blank?

    errors.add(:carer, 'must belong to the relationship household') if carer && carer.household_id != household_id
    errors.add(:patient, 'must belong to the relationship household') if patient && patient.household_id != household_id
  end
end
