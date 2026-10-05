# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Household do
  def household_bundle(name:)
    household = create_household(name)
    location = household.locations.find_by!(name: 'Home')
    medication = create_medication(household, location)
    dosage = create_dosage(household, medication)
    person = create_person(household, "#{name} Patient")

    [household, person, location, medication, dosage]
  end

  def create_household(name)
    account = Account.create!(email: "#{name.parameterize}-owner@example.test", status: :verified)
    Household.create_with_owner!(
      name: name,
      owner_account: account,
      owner_person_attributes: {
        name: "#{name} Owner",
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      }
    )
  end

  def create_medication(household, location)
    household.medications.create!(
      name: 'Paracetamol',
      location: location,
      reorder_threshold: 10
    )
  end

  def create_dosage(household, medication)
    medication.dosage_records.create!(
      household: household,
      amount: 5,
      unit: 'ml',
      frequency: 'Daily',
      default_max_daily_doses: 4,
      default_min_hours_between_doses: 4,
      default_dose_cycle: :daily
    )
  end

  def create_person(household, name)
    household.people.create!(
      name: name,
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
  end

  def schedule_for(household:, person:, medication:, dosage:)
    Schedule.create!(
      household: household,
      person: person,
      medication: medication,
      source_dosage_option: dosage,
      start_date: Time.zone.today,
      end_date: 1.week.from_now.to_date,
      dose_amount: 5,
      dose_unit: 'ml',
      frequency: 'Daily'
    )
  end

  def create_medication_take(household:, schedule:)
    MedicationTake.create!(
      household: household,
      schedule: schedule,
      taken_at: Time.current,
      dose_amount: 5,
      dose_unit: 'ml'
    )
  end

  it 'rejects medication inventory stored at a foreign household location' do
    household, = household_bundle(name: 'Medication Tenant A')
    _, _, foreign_location = household_bundle(name: 'Medication Tenant B')

    expect do
      Medication.create!(
        household: household,
        name: 'Foreign Stock',
        location: foreign_location,
        reorder_threshold: 10
      )
    end.to raise_error(ActiveRecord::InvalidForeignKey)
  end

  it 'rejects schedules that join a person to a foreign household medication' do
    household, person = household_bundle(name: 'Schedule Tenant A')
    _, _, _, foreign_medication, foreign_dosage = household_bundle(name: 'Schedule Tenant B')

    expect do
      schedule_for(
        household: household,
        person: person,
        medication: foreign_medication,
        dosage: foreign_dosage
      )
    end.to raise_error(ActiveRecord::InvalidForeignKey)
  end

  it 'rejects direct person medication links across households' do
    household, person = household_bundle(name: 'Person Medication Tenant A')
    _, _, _, foreign_medication, foreign_dosage = household_bundle(name: 'Person Medication Tenant B')

    expect do
      PersonMedication.create!(
        household: household,
        person: person,
        medication: foreign_medication,
        source_dosage_option: foreign_dosage,
        dose_amount: 5,
        dose_unit: 'ml',
        administration_kind: :as_needed
      )
    end.to raise_error(ActiveRecord::InvalidForeignKey)
  end

  it 'rejects person-location associations across households' do
    household, person = household_bundle(name: 'Location Tenant A')
    _, _, foreign_location = household_bundle(name: 'Location Tenant B')

    expect do
      LocationMembership.new(
        household: household,
        person: person,
        location: foreign_location
      ).save!(validate: false)
    end.to raise_error(ActiveRecord::InvalidForeignKey)
  end

  it 'rejects medication takes whose household does not match the source schedule' do
    household, = household_bundle(name: 'Take Tenant A')
    _, foreign_person, _, foreign_medication, foreign_dosage = household_bundle(name: 'Take Tenant B')

    foreign_schedule = schedule_for(
      household: foreign_medication.household,
      person: foreign_person,
      medication: foreign_medication,
      dosage: foreign_dosage
    )

    expect do
      create_medication_take(household: household, schedule: foreign_schedule)
    end.to raise_error(ActiveRecord::InvalidForeignKey)
  end
end
