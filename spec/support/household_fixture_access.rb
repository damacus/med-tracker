# frozen_string_literal: true

module HouseholdFixtureAccess
  def active_spec_household
    return @browser_household if instance_variable_defined?(:@browser_household) && @browser_household

    return Household.find_by!(slug: default_request_household_slug) if respond_to?(:default_request_household_slug)

    return Household.find_by!(slug: default_browser_household_slug) if respond_to?(:default_browser_household_slug)

    Current.household || Household.first
  end

  def household_person(person)
    household_fixture_record(person)
  end

  def household_schedule(schedule)
    household_fixture_record(schedule)
  end

  def household_person_medication(person_medication)
    household_fixture_record(person_medication)
  end

  def household_medication(medication)
    household_fixture_record(medication) do |scope|
      scope.where(
        name: medication.name,
        dosage_amount: medication.dosage_amount,
        dosage_unit: medication.dosage_unit
      ).first
    end
  end

  def household_location(location)
    household_fixture_record(location) do |scope|
      scope.where('LOWER(name) = ?', location.name.downcase).first
    end
  end

  def household_fixture_record(record)
    scope = record.class.where(household: active_spec_household)
    scope.find_by(id: record.id) || (block_given? ? yield(scope) : nil) || scope.find(record.id)
  end
end

RSpec.configure do |config|
  config.include HouseholdFixtureAccess, type: :request
  config.include HouseholdFixtureAccess, type: :system
  config.include HouseholdFixtureAccess, type: :feature
end
