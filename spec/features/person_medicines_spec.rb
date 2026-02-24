# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Person Medicines', type: :system do
  fixtures :accounts, :people, :locations, :medicines, :users, :person_medicines

  let(:person) { people(:john) }
  let(:user) { users(:john) }

  before do
    login_as(user)
  end

  describe 'adding a non-prescription medicine' do
    let(:new_medicine) { medicines(:vitamin_c) }

    it 'allows adding a vitamin/OTC medicine without a prescription' do
      visit person_path(person)

      expect(page).to have_content('My Medicines')

      click_link 'Add Medicine'

      select new_medicine.name, from: 'person_medicine[medicine_id]'
      fill_in 'person_medicine[notes]', with: 'Take with breakfast'
      fill_in 'person_medicine[max_daily_doses]', with: '1'

      click_button 'Add Medicine'

      expect(page).to have_content('Medicine added successfully')
      expect(page).to have_content(new_medicine.name)
      expect(page).to have_content('Take with breakfast')
    end
  end

  describe 'recording medicine takes' do
    let(:person_medicine) { person_medicines(:john_vitamin_d) }

    it 'allows recording a medicine take' do
      person_medicine.update!(max_daily_doses: 3, min_hours_between_doses: nil)
      person_medicine.medication_takes.delete_all

      visit person_path(person)

      within("#person_medicine_#{person_medicine.id}") do
        click_button '💊 Take'
      end

      expect(page).to have_content('Medicine taken successfully')
    end

    it 'disables the take button when max daily doses reached' do
      person_medicine.update!(max_daily_doses: 2, min_hours_between_doses: nil)
      person_medicine.medication_takes.delete_all
      2.times do
        MedicationTake.create!(
          person_medicine: person_medicine,
          taken_at: Time.current,
          amount_ml: 5
        )
      end

      visit person_path(person)

      within("#person_medicine_#{person_medicine.id}") do
        expect(page).to have_button('💊 Take', disabled: true)
      end
    end

    it 'disables the take button when minimum hours not passed' do
      person_medicine.update!(max_daily_doses: nil, min_hours_between_doses: 6)
      person_medicine.medication_takes.delete_all
      MedicationTake.create!(
        person_medicine: person_medicine,
        taken_at: 2.hours.ago,
        amount_ml: 5
      )

      visit person_path(person)

      within("#person_medicine_#{person_medicine.id}") do
        expect(page).to have_button('💊 Take', disabled: true)
      end
    end
  end

  describe 'viewing today\'s doses on card' do
    let(:person_medicine) { person_medicines(:john_vitamin_d) }

    let!(:take) do
      MedicationTake.create!(
        person_medicine: person_medicine,
        taken_at: Time.current,
        amount_ml: 5
      )
    end

    it 'displays today\'s doses on the medicine card' do
      visit person_path(person)

      within("#person_medicine_#{person_medicine.id}") do
        expect(page).to have_content("TODAY'S DOSES")
        expect(page).to have_content(take.taken_at.strftime('%l:%M %p').strip)
        expect(page).to have_content('5ML')
      end
    end
  end
end
