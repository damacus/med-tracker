# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Person Medications Authorization' do
  fixtures :accounts, :people, :users, :locations, :medications, :carer_relationships

  before do
    driven_by(:playwright)
  end

  let(:admin) { users(:admin) }
  let(:doctor) { users(:doctor) }
  let(:nurse) { users(:nurse) }
  let(:carer) { users(:carer) }
  let(:parent) { users(:parent) }
  let(:linked_child) { people(:child_user_person) }
  let(:unlinked_child) { people(:child_patient) }
  let(:assigned_patient) { people(:child_patient) }
  let(:unrelated_person) { people(:john) }

  describe 'adding medications' do
    let(:medication) { medications(:vitamin_d) }

    it 'allows administrators to add medications to any person' do
      login_as(admin)
      visit person_path(assigned_patient)

      expect(page).to have_link('Add Medication')
      click_link 'Add Medication'
      select medication.name, from: 'Medication'
      fill_in 'Notes', with: 'Test notes'
      click_button 'Add Medication'

      expect(page).to have_content('Medication added successfully')
    end

    it 'denies doctors ability to add medications' do
      login_as(doctor)
      visit person_path(assigned_patient)

      expect(page).to have_content('My Medications')
      expect(page).to have_no_link('Add Medication')
    end

    it 'denies nurses ability to add medications' do
      login_as(nurse)
      visit person_path(assigned_patient)

      expect(page).to have_content('My Medications')
      expect(page).to have_no_link('Add Medication')
    end

    it 'denies carers ability to add medications to unrelated people' do
      login_as(carer)
      visit person_path(unrelated_person)

      expect(page).to have_content('You are not authorized to perform this action')
    end

    it 'denies carers ability to add medications to their assigned patients' do
      login_as(carer)
      visit person_path(assigned_patient)

      expect(page).to have_content('My Medications')
      expect(page).to have_no_link('Add Medication')
    end

    it 'allows parents to add medications to linked children' do
      login_as(parent)
      visit person_path(linked_child)

      expect(page).to have_link('Add Medication')
      click_link 'Add Medication'
      select medication.name, from: 'Medication'
      fill_in 'Notes', with: 'Parent-added medication'
      click_button 'Add Medication'

      expect(page).to have_content('Medication added successfully')
      expect(page).to have_content(medication.name)
    end

    it 'denies parents ability to add medications to unlinked children' do
      login_as(parent)
      visit person_path(unlinked_child)

      expect(page).to have_content('You are not authorized to perform this action')
    end
  end

  describe 'taking medications' do
    let(:medication) { medications(:vitamin_d) }
    let!(:person_medication) do
      PersonMedication.create!(
        person: assigned_patient,
        medication: medication,
        notes: 'Test medication'
      )
    end

    it 'allows carers to take medication for assigned patients' do
      login_as(carer)
      visit person_path(assigned_patient)

      within("#person_medication_#{person_medication.id}") do
        expect(page).to have_button('💊 Give')
        click_button '💊 Give'
      end

      expect(page).to have_content('Medication taken successfully')
    end

    it 'denies doctors ability to take medications' do
      login_as(doctor)
      visit person_path(assigned_patient)

      within("#person_medication_#{person_medication.id}") do
        expect(page).to have_no_button('💊 Give')
      end
    end

    it 'denies nurses ability to take medications' do
      login_as(nurse)
      visit person_path(assigned_patient)

      within("#person_medication_#{person_medication.id}") do
        expect(page).to have_no_button('💊 Give')
      end
    end

    it 'denies carers ability to take medication for unrelated people' do
      # Use a different medication to avoid validation error
      different_medication = medications(:ibuprofen)
      PersonMedication.create!(
        person: unrelated_person,
        medication: different_medication,
        notes: 'Unrelated medication'
      )

      login_as(carer)
      visit person_path(unrelated_person)

      expect(page).to have_content('You are not authorized to perform this action')
    end
  end

  describe 'removing medications' do
    let(:medication) { medications(:vitamin_d) }
    let!(:person_medication) do
      PersonMedication.create!(
        person: assigned_patient,
        medication: medication,
        notes: 'Test medication'
      )
    end

    it 'allows administrators to remove medications from any person' do
      login_as(admin)
      visit person_path(assigned_patient)

      within("#person_medication_#{person_medication.id}") do
        # Trigger is an icon button
        delete_button = find("[data-testid='delete-person-medication-#{person_medication.id}']")
        expect(delete_button).to have_css('svg')
        delete_button.click
      end

      # Should show confirmation dialog
      expect(page).to have_content('Remove Medication')
    end

    it 'denies doctors ability to remove medications' do
      login_as(doctor)
      visit person_path(assigned_patient)

      within("#person_medication_#{person_medication.id}") do
        expect(page).to have_no_css('button svg')
      end
    end

    it 'denies nurses ability to remove medications' do
      login_as(nurse)
      visit person_path(assigned_patient)

      within("#person_medication_#{person_medication.id}") do
        expect(page).to have_no_css('button svg')
      end
    end

    it 'denies carers ability to remove medications from assigned patients' do
      login_as(carer)
      visit person_path(assigned_patient)

      within("#person_medication_#{person_medication.id}") do
        expect(page).to have_no_css('button svg')
      end
    end
  end

  describe 'reordering medications' do
    let!(:parent_first_medication) do
      PersonMedication.create!(
        person: linked_child,
        medication: medications(:vitamin_d),
        notes: 'First medication'
      )
    end
    let!(:parent_second_medication) do
      PersonMedication.create!(
        person: linked_child,
        medication: medications(:vitamin_c),
        notes: 'Second medication'
      )
    end

    let!(:carer_first_medication) do
      PersonMedication.create!(
        person: assigned_patient,
        medication: medications(:ibuprofen),
        notes: 'Carer first'
      )
    end
    let!(:carer_second_medication) do
      PersonMedication.create!(
        person: assigned_patient,
        medication: medications(:aspirin),
        notes: 'Carer second'
      )
    end

    it 'shows reorder controls to parents for linked children' do
      login_as(parent)
      visit person_path(linked_child)

      expect(page).to have_css("[data-testid='move-up-person-medication-#{parent_first_medication.id}']")
      expect(page).to have_css("[data-testid='move-down-person-medication-#{parent_second_medication.id}']")
    end

    it 'does not show reorder controls to carers for assigned patients' do
      login_as(carer)
      visit person_path(assigned_patient)

      expect(page).to have_content('My Medications')
      expect(page).to have_no_css("[data-testid='move-up-person-medication-#{carer_first_medication.id}']")
      expect(page).to have_no_css("[data-testid='move-down-person-medication-#{carer_second_medication.id}']")
    end
  end

  describe 'editing medications' do
    let(:medication) { medications(:vitamin_d) }
    let!(:person_medication) do
      PersonMedication.create!(
        person: linked_child,
        medication: medication,
        notes: 'Original notes',
        max_daily_doses: 3
      )
    end

    it 'allows parents to edit medications for linked children' do
      login_as(parent)
      visit person_path(linked_child)

      within("#person_medication_#{person_medication.id}") do
        find("[data-testid='edit-person-medication-#{person_medication.id}']").click
      end

      expect(page).to have_content('Edit Medication for')
      fill_in 'Notes', with: 'Updated notes'
      fill_in 'Max daily doses', with: '5'
      click_button 'Save Changes'

      expect(page).to have_content('Medication updated successfully')
      expect(page).to have_content('Updated notes')
    end

    it 'allows administrators to edit medications for any person' do
      login_as(admin)
      visit person_path(linked_child)

      within("#person_medication_#{person_medication.id}") do
        find("[data-testid='edit-person-medication-#{person_medication.id}']").click
      end

      expect(page).to have_content('Edit Medication for')
      fill_in 'Notes', with: 'Admin edited notes'
      click_button 'Save Changes'

      expect(page).to have_content('Medication updated successfully')
    end

    it 'does not show edit button to carers for assigned patients' do
      carer_medication = PersonMedication.create!(person: assigned_patient, medication: medications(:ibuprofen))

      login_as(carer)
      visit person_path(assigned_patient)

      within("#person_medication_#{carer_medication.id}") do
        expect(page).to have_no_css("[data-testid='edit-person-medication-#{carer_medication.id}']")
      end
    end
  end

  describe 'viewing medications' do
    let(:medication) { medications(:vitamin_d) }

    it 'allows administrators to view medications for any person' do
      PersonMedication.create!(
        person: assigned_patient,
        medication: medication,
        notes: 'Test medication'
      )

      login_as(admin)
      visit person_path(assigned_patient)

      expect(page).to have_content('My Medications')
      expect(page).to have_content(medication.name)
    end

    it 'allows doctors to view medications for any person' do
      PersonMedication.create!(
        person: assigned_patient,
        medication: medication,
        notes: 'Test medication'
      )

      login_as(doctor)
      visit person_path(assigned_patient)

      expect(page).to have_content('My Medications')
      expect(page).to have_content(medication.name)
    end

    it 'allows nurses to view medications for any person' do
      PersonMedication.create!(
        person: assigned_patient,
        medication: medication,
        notes: 'Test medication'
      )

      login_as(nurse)
      visit person_path(assigned_patient)

      expect(page).to have_content('My Medications')
      expect(page).to have_content(medication.name)
    end

    it 'allows carers to view medications for assigned patients' do
      PersonMedication.create!(
        person: assigned_patient,
        medication: medication,
        notes: 'Test medication'
      )

      login_as(carer)
      visit person_path(assigned_patient)

      expect(page).to have_content('My Medications')
      expect(page).to have_content(medication.name)
    end

    it 'denies carers ability to view medications for unrelated people' do
      PersonMedication.find_or_create_by!(
        person: unrelated_person,
        medication: medications(:ibuprofen)
      ) do |pm|
        pm.notes = 'Unrelated medication'
      end

      login_as(carer)
      visit person_path(unrelated_person)

      expect(page).to have_content('You are not authorized to perform this action')
    end
  end
end
