# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Person Medicines Authorization' do
  fixtures :accounts, :people, :users, :medicines, :carer_relationships

  before do
    driven_by(:playwright)
  end

  let(:admin) { users(:admin) }
  let(:doctor) { users(:doctor) }
  let(:nurse) { users(:nurse) }
  let(:carer) { users(:jane) }
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
        expect(page).to have_css('button svg')
        find('button svg').click
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
