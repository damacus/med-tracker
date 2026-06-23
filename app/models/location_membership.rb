# frozen_string_literal: true

class LocationMembership < ApplicationRecord
  has_paper_trail

  belongs_to :household, optional: true
  belongs_to :location
  belongs_to :person

  before_validation :assign_household

  validates :person_id, uniqueness: { scope: :location_id }
  validate :linked_records_must_belong_to_household, if: :household_id?

  private

  def assign_household
    self.household ||= location&.household || person&.household
  end

  def linked_records_must_belong_to_household
    errors.add(:location, 'must belong to the same household') if location&.household_id != household_id
    errors.add(:person, 'must belong to the same household') if person&.household_id != household_id
  end
end
