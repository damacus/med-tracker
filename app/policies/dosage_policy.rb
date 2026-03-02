# frozen_string_literal: true

class DosagePolicy < ApplicationPolicy
  delegate :show?, to: :medication_policy
  def create?  = medication_policy.update?
  def new?     = create?
  delegate :update?, to: :medication_policy
  def edit?    = update?
  def destroy? = medication_policy.update?

  private

  def medication_policy
    MedicationPolicy.new(user, record.medication)
  end
end
