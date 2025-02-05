require "system/playwright/test_helper"

class MedicinesTest < ApplicationSystemTestCase
  test "creating a new medicine" do
    visit medicines_path
    click_on "Add Medicine"

    fill_in "Name", with: "Ibuprofen"
    fill_in "Description", with: "Pain reliever and fever reducer"
    fill_in "Standard dosage", with: "200-400mg every 4-6 hours"
    click_on "Create Medicine"

    assert_text "Medicine was successfully created"
    assert_text "Ibuprofen"
    assert_text "Pain reliever and fever reducer"
  end

  test "editing an existing medicine" do
    # Create a medicine first
    medicine = Medicine.create!(
      name: "Aspirin",
      description: "Pain reliever",
      standard_dosage: "325-650mg every 4 hours"
    )

    visit medicines_path
    within("#medicine_#{medicine.id}") do
      click_on "Edit"
    end

    fill_in "Description", with: "Pain reliever and blood thinner"
    click_on "Update Medicine"

    assert_text "Medicine was successfully updated"
    assert_text "Pain reliever and blood thinner"
  end

  test "deleting a medicine" do
    medicine = Medicine.create!(
      name: "Test Medicine",
      description: "To be deleted",
      standard_dosage: "1 tablet daily"
    )

    visit medicines_path
    # Find and click the delete button
    within("#medicine_#{medicine.id}") do
      click_button "Delete"
    end

    assert_text "Medicine was successfully deleted"
    refute_text "Test Medicine"
  end

  test "viewing medicine details" do
    medicine = Medicine.create!(
      name: "Vitamin D3",
      description: "Vitamin D supplement",
      standard_dosage: "1000 IU daily",
      warnings: "Take with food for better absorption"
    )

    visit medicine_path(medicine)

    assert_text "Vitamin D3"
    assert_text "Vitamin D supplement"
    assert_text "1000 IU daily"
    assert_text "Take with food for better absorption"
  end
end
