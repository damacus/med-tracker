# frozen_string_literal: true

require 'system/playwright/test_helper'

class MedicinesTest < ApplicationSystemTestCase
  def setup
    @medicine = medicines(:paracetamol)
    # Clean up any existing dependencies
    @medicine.prescriptions.each do |prescription|
      prescription.take_medicines.destroy_all
    end
    @medicine.prescriptions.destroy_all
  end

  test 'creating a new medicine with dosage options' do
    visit medicines_path
    click_on 'Add Medicine'

    fill_in 'Name', with: 'New Medicine'
    fill_in 'Description', with: 'Test description'
    fill_in 'Dosage', with: '500'
    select 'mg', from: 'medicine[unit]'

    # Add first dosage option
    click_button 'Add Option'
    assert_selector "[data-dosage-options-target='container'] input[type='number']"
    within("[data-dosage-options-target='container']") do
      first("input[type='number']").fill_in with: '250'
    end

    # Add second dosage option
    click_button 'Add Option'
    assert_selector "[data-dosage-options-target='container'] input[type='number']", count: 2
    within("[data-dosage-options-target='container']") do
      all("input[type='number']").last.fill_in with: '750'
    end
    fill_in 'Warnings', with: 'Test warnings'
    click_on 'Create Medicine'

    assert_text 'Medicine was successfully created'
    assert_text 'New Medicine'
    assert_text '500.0 mg'
  end

  test "editing an existing medicine's dosage options" do
    visit edit_medicine_path(@medicine)

    # Add a new dosage option
    click_button 'Add Option'
    assert_selector "[data-dosage-options-target='container'] input[type='number']"
    within("[data-dosage-options-target='container']") do
      first("input[type='number']").fill_in with: '750'
    end

    # Remove the dosage option
    within("[data-dosage-options-target='container']") do
      first('button', text: 'Remove').click
    end

    click_on 'Update Medicine'
    assert_text 'Medicine was successfully updated'
  end

  test 'validation errors for medicine with invalid dosage options' do
    visit new_medicine_path

    fill_in 'Name', with: 'Test Medicine'
    fill_in 'Description', with: 'Test description'
    fill_in 'Dosage', with: '500'
    select 'mg', from: 'medicine[unit]'

    # Try to add invalid dosage options
    click_button 'Add Option'
    assert_selector "[data-dosage-options-target='container'] input[type='number']"
    within("[data-dosage-options-target='container']") do
      first("input[type='number']").fill_in with: '0'
    end

    click_on 'Create Medicine'

    # Wait for the error message to appear
    assert_selector '.form__errors'
    assert_text 'Dosage options amount must be greater than 0'
  end

  test 'deleting a medicine also deletes its dosage options' do
    visit medicines_path

    # Clean up any existing dependencies
    @medicine.prescriptions.each do |prescription|
      prescription.take_medicines.destroy_all
    end
    @medicine.prescriptions.destroy_all

    # Now try to delete
    accept_confirm do
      click_button 'Delete'
    end

    assert_text 'Medicine was successfully deleted'
    assert_equal 0, DosageOption.where(medicine_id: @medicine.id).count
    refute_text 'Test Medicine'
  end

  test 'viewing medicine details' do
    medicine = Medicine.create!(
      name: 'Vitamin D3',
      description: 'Vitamin D supplement',
      dosage: '1000',
      unit: 'IU',
      warnings: 'Take with food for better absorption'
    )

    visit medicine_path(medicine)

    assert_text 'Vitamin D3'
    assert_text 'Vitamin D supplement'
    assert_text '1000'
    assert_text 'Take with food for better absorption'
  end
end
