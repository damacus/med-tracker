# frozen_string_literal: true

class InvitationDependent < ApplicationRecord
  belongs_to :invitation
  belongs_to :dependent, class_name: 'Person'

  validates :dependent_id, uniqueness: { scope: :invitation_id }
  validate :dependent_must_require_carer

  private

  def dependent_must_require_carer
    return if dependent&.person_type.in?(%w[minor dependent_adult]) && dependent.has_capacity == false

    errors.add(:dependent, 'must be a child or dependent adult without capacity')
  end
end
