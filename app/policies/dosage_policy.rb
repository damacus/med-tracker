# frozen_string_literal: true

class DosagePolicy < ApplicationPolicy
  def show?    = medication_policy.show?
  def create?  = medication_policy.update?
  def new?     = create?
  def update?  = medication_policy.update?
  def edit?    = update?
  def destroy? = medication_policy.update?

  private

  def medication_policy
    MedicationPolicy.new(user, record.medication)
  end
end
