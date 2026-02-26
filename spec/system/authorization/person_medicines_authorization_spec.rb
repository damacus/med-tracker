# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Person Medicines Authorization' do
  fixtures :accounts, :people, :users, :locations, :medicines, :carer_relationships

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

  describe 'adding medicines' do
    let(:medicine) { medicines(:vitamin_d) }

    it 'allows administrators to add medicines to any person' do
      login_as(admin)
      visit person_path(assigned_patient)

      expect(page).to have_link('Add Medicine')
      click_link 'Add Medicine'
      select medicine.name, from: 'Medicine'
      fill_in 'Notes', with: 'Test notes'
      click_button 'Add Medicine'

      expect(page).to have_content('Medicine added successfully')
    end

    it 'denies doctors ability to add medicines' do
      login_as(doctor)
      visit person_path(assigned_patient)

      expect(page).to have_content('My Medicines')
      expect(page).to have_no_link('Add Medicine')
    end

    it 'denies nurses ability to add medicines' do
      login_as(nurse)
      visit person_path(assigned_patient)

      expect(page).to have_content('My Medicines')
      expect(page).to have_no_link('Add Medicine')
    end

    it 'denies carers ability to add medicines to unrelated people' do
      login_as(carer)
      visit person_path(unrelated_person)

      expect(page).to have_content('You are not authorized to perform this action')
    end

    it 'denies carers ability to add medicines to their assigned patients' do
      login_as(carer)
      visit person_path(assigned_patient)

      expect(page).to have_content('My Medicines')
      expect(page).to have_no_link('Add Medicine')
    end

    it 'allows parents to add medicines to linked children' do
      login_as(parent)
      visit person_path(linked_child)

      expect(page).to have_link('Add Medicine')
      click_link 'Add Medicine'
      select medicine.name, from: 'Medicine'
      fill_in 'Notes', with: 'Parent-added medicine'
      click_button 'Add Medicine'

      expect(page).to have_content('Medicine added successfully')
      expect(page).to have_content(medicine.name)
    end

    it 'denies parents ability to add medicines to unlinked children' do
      login_as(parent)
      visit person_path(unlinked_child)

      expect(page).to have_content('You are not authorized to perform this action')
    end
  end

  describe 'taking medicines' do
    let(:medicine) { medicines(:vitamin_d) }
    let!(:person_medicine) do
      PersonMedicine.create!(
        person: assigned_patient,
        medicine: medicine,
        notes: 'Test medicine'
      )
    end

    it 'allows carers to take medicine for assigned patients' do
      login_as(carer)
      visit person_path(assigned_patient)

      within("#person_medicine_#{person_medicine.id}") do
        expect(page).to have_button('💊 Give')
        click_button '💊 Give'
      end

      expect(page).to have_content('Medicine taken successfully')
    end

    it 'denies doctors ability to take medicines' do
      login_as(doctor)
      visit person_path(assigned_patient)

      within("#person_medicine_#{person_medicine.id}") do
        expect(page).to have_no_button('💊 Give')
      end
    end

    it 'denies nurses ability to take medicines' do
      login_as(nurse)
      visit person_path(assigned_patient)

      within("#person_medicine_#{person_medicine.id}") do
        expect(page).to have_no_button('💊 Give')
      end
    end

    it 'denies carers ability to take medicine for unrelated people' do
      # Use a different medicine to avoid validation error
      different_medicine = medicines(:ibuprofen)
      PersonMedicine.create!(
        person: unrelated_person,
        medicine: different_medicine,
        notes: 'Unrelated medicine'
      )

      login_as(carer)
      visit person_path(unrelated_person)

      expect(page).to have_content('You are not authorized to perform this action')
    end
  end

  describe 'removing medicines' do
    let(:medicine) { medicines(:vitamin_d) }
    let!(:person_medicine) do
      PersonMedicine.create!(
        person: assigned_patient,
        medicine: medicine,
        notes: 'Test medicine'
      )
    end

    it 'allows administrators to remove medicines from any person' do
      login_as(admin)
      visit person_path(assigned_patient)

      within("#person_medicine_#{person_medicine.id}") do
        # Trigger is an icon button
        delete_button = find("[data-testid='delete-person-medicine-#{person_medicine.id}']")
        expect(delete_button).to have_css('svg')
        delete_button.click
      end

      # Should show confirmation dialog
      expect(page).to have_content('Remove Medicine')
    end

    it 'denies doctors ability to remove medicines' do
      login_as(doctor)
      visit person_path(assigned_patient)

      within("#person_medicine_#{person_medicine.id}") do
        expect(page).to have_no_css('button svg')
      end
    end

    it 'denies nurses ability to remove medicines' do
      login_as(nurse)
      visit person_path(assigned_patient)

      within("#person_medicine_#{person_medicine.id}") do
        expect(page).to have_no_css('button svg')
      end
    end

    it 'denies carers ability to remove medicines from assigned patients' do
      login_as(carer)
      visit person_path(assigned_patient)

      within("#person_medicine_#{person_medicine.id}") do
        expect(page).to have_no_css('button svg')
      end
    end
  end

  describe 'reordering medicines' do
    let!(:parent_first_medicine) do
      PersonMedicine.create!(
        person: linked_child,
        medicine: medicines(:vitamin_d),
        notes: 'First medicine'
      )
    end
    let!(:parent_second_medicine) do
      PersonMedicine.create!(
        person: linked_child,
        medicine: medicines(:vitamin_c),
        notes: 'Second medicine'
      )
    end

    let!(:carer_first_medicine) do
      PersonMedicine.create!(
        person: assigned_patient,
        medicine: medicines(:ibuprofen),
        notes: 'Carer first'
      )
    end
    let!(:carer_second_medicine) do
      PersonMedicine.create!(
        person: assigned_patient,
        medicine: medicines(:aspirin),
        notes: 'Carer second'
      )
    end

    it 'shows reorder controls to parents for linked children' do
      login_as(parent)
      visit person_path(linked_child)

      expect(page).to have_css("[data-testid='move-up-person-medicine-#{parent_first_medicine.id}']")
      expect(page).to have_css("[data-testid='move-down-person-medicine-#{parent_second_medicine.id}']")
    end

    it 'does not show reorder controls to carers for assigned patients' do
      login_as(carer)
      visit person_path(assigned_patient)

      expect(page).to have_content('My Medicines')
      expect(page).to have_no_css("[data-testid='move-up-person-medicine-#{carer_first_medicine.id}']")
      expect(page).to have_no_css("[data-testid='move-down-person-medicine-#{carer_second_medicine.id}']")
    end
  end

  describe 'editing medicines' do
    let(:medicine) { medicines(:vitamin_d) }
    let!(:person_medicine) do
      PersonMedicine.create!(
        person: linked_child,
        medicine: medicine,
        notes: 'Original notes',
        max_daily_doses: 3
      )
    end

    it 'allows parents to edit medicines for linked children' do
      login_as(parent)
      visit person_path(linked_child)

      within("#person_medicine_#{person_medicine.id}") do
        find("[data-testid='edit-person-medicine-#{person_medicine.id}']").click
      end

      expect(page).to have_content('Edit Medicine for')
      fill_in 'Notes', with: 'Updated notes'
      fill_in 'Max daily doses', with: '5'
      click_button 'Save Changes'

      expect(page).to have_content('Medicine updated successfully')
      expect(page).to have_content('Updated notes')
    end

    it 'allows administrators to edit medicines for any person' do
      login_as(admin)
      visit person_path(linked_child)

      within("#person_medicine_#{person_medicine.id}") do
        find("[data-testid='edit-person-medicine-#{person_medicine.id}']").click
      end

      expect(page).to have_content('Edit Medicine for')
      fill_in 'Notes', with: 'Admin edited notes'
      click_button 'Save Changes'

      expect(page).to have_content('Medicine updated successfully')
    end

    it 'does not show edit button to carers for assigned patients' do
      carer_medicine = PersonMedicine.create!(person: assigned_patient, medicine: medicines(:ibuprofen))

      login_as(carer)
      visit person_path(assigned_patient)

      within("#person_medicine_#{carer_medicine.id}") do
        expect(page).to have_no_css("[data-testid='edit-person-medicine-#{carer_medicine.id}']")
      end
    end
  end

  describe 'viewing medicines' do
    let(:medicine) { medicines(:vitamin_d) }

    it 'allows administrators to view medicines for any person' do
      PersonMedicine.create!(
        person: assigned_patient,
        medicine: medicine,
        notes: 'Test medicine'
      )

      login_as(admin)
      visit person_path(assigned_patient)

      expect(page).to have_content('My Medicines')
      expect(page).to have_content(medicine.name)
    end

    it 'allows doctors to view medicines for any person' do
      PersonMedicine.create!(
        person: assigned_patient,
        medicine: medicine,
        notes: 'Test medicine'
      )

      login_as(doctor)
      visit person_path(assigned_patient)

      expect(page).to have_content('My Medicines')
      expect(page).to have_content(medicine.name)
    end

    it 'allows nurses to view medicines for any person' do
      PersonMedicine.create!(
        person: assigned_patient,
        medicine: medicine,
        notes: 'Test medicine'
      )

      login_as(nurse)
      visit person_path(assigned_patient)

      expect(page).to have_content('My Medicines')
      expect(page).to have_content(medicine.name)
    end

    it 'allows carers to view medicines for assigned patients' do
      PersonMedicine.create!(
        person: assigned_patient,
        medicine: medicine,
        notes: 'Test medicine'
      )

      login_as(carer)
      visit person_path(assigned_patient)

      expect(page).to have_content('My Medicines')
      expect(page).to have_content(medicine.name)
    end

    it 'denies carers ability to view medicines for unrelated people' do
      PersonMedicine.find_or_create_by!(
        person: unrelated_person,
        medicine: medicines(:ibuprofen)
      ) do |pm|
        pm.notes = 'Unrelated medicine'
      end

      login_as(carer)
      visit person_path(unrelated_person)

      expect(page).to have_content('You are not authorized to perform this action')
    end
  end
end
