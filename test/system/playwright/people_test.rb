require "system/playwright/test_helper"

class PeopleTest < ApplicationSystemTestCase
  test "creating a new person" do
    visit people_path
    click_on "Add Person"

    fill_in "Name", with: "John Doe"
    fill_in "Date of birth", with: "1990-01-01"
    click_on "Create Person"

    assert_text "Person was successfully created"
    assert_text "John Doe"
    assert_text "January 01, 1990"
  end

  test "adding a prescription to a person" do
    person = Person.create!(
      name: "Jane Doe",
      date_of_birth: "1985-05-15"
    )

    medicine = Medicine.create!(
      name: "Ibuprofen",
      description: "Pain reliever",
      standard_dosage: "200-400mg every 4-6 hours"
    )

    visit person_path(person)
    click_on "Add Prescription"

    select "Ibuprofen", from: "Medicine"
    fill_in "Dosage", with: "200mg"
    fill_in "Frequency", with: "Every 6 hours"
    fill_in "Start date", with: Date.current.to_s
    fill_in "Notes", with: "Take with food"
    click_on "Create Prescription"

    assert_text "Prescription was successfully created"
    assert_text "Ibuprofen"
    assert_text "200mg"
    assert_text "Every 6 hours"
    assert_text "Take with food"
  end

  test "viewing person's prescriptions" do
    person = Person.create!(
      name: "Bob Smith",
      date_of_birth: "1975-03-20"
    )

    medicine = Medicine.create!(
      name: "Vitamin D",
      description: "Vitamin D supplement",
      standard_dosage: "1000 IU daily"
    )

    prescription = person.prescriptions.create!(
      medicine: medicine,
      dosage: "2000 IU",
      frequency: "Once daily",
      start_date: Date.current,
      notes: "Take with breakfast"
    )

    visit person_path(person)

    assert_text "Bob Smith"
    assert_text "Vitamin D"
    assert_text "2000 IU"
    assert_text "Once daily"
    assert_text "Take with breakfast"
  end

  test "editing a person's prescription" do
    person = Person.create!(
      name: "Alice Johnson",
      date_of_birth: "1995-12-10"
    )

    medicine = Medicine.create!(
      name: "Aspirin",
      description: "Pain reliever",
      standard_dosage: "325-650mg every 4 hours"
    )

    prescription = person.prescriptions.create!(
      medicine: medicine,
      dosage: "325mg",
      frequency: "Every 6 hours",
      start_date: Date.current,
      notes: "Original note"
    )

    visit person_path(person)
    within("#prescription_#{prescription.id}") do
      click_on "Edit"
    end

    fill_in "Frequency", with: "Every 8 hours"
    fill_in "Notes", with: "Updated: Take after meals"
    click_on "Update Prescription"

    assert_text "Prescription was successfully updated"
    assert_text "Every 8 hours"
    assert_text "Updated: Take after meals"
  end
end
