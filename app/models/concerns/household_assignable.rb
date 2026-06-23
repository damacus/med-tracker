# frozen_string_literal: true

module HouseholdAssignable
  private

  def assign_household
    self.household ||= household_source
  end

  def household_source
    person&.household || medication&.household || source_dosage_option&.household
  end
end
