# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Person Medications', type: :system do
  fixtures :accounts, :people, :locations, :medications, :users, :person_medications

  let(:person) { people(:john) }
  let(:user) { users(:john) }

  before do
    login_as(user)
  end

  describe 'adding a non-schedule medication' do
    let(:new_medication) { medications(:vitamin_c) }

    it 'allows adding a vitamin/OTC medication without a schedule' do
      visit person_path(person)

      expect(page).to have_content('My Medications')

      click_link 'Add Medication'

      select new_medication.name, from: 'person_medication[medication_id]'
      fill_in 'person_medication[notes]', with: 'Take with breakfast'
      fill_in 'person_medication[max_daily_doses]', with: '1'

      click_button 'Add Medication'

      expect(page).to have_content('Medication added successfully')
      expect(page).to have_content(new_medication.name)
      expect(page).to have_content('Take with breakfast')
    end
  end

  describe 'recording medication takes' do
    let(:person_medication) { person_medications(:john_vitamin_d) }

    it 'allows recording a medication take' do
      person_medication.update!(max_daily_doses: 3, min_hours_between_doses: nil)
      person_medication.medication_takes.delete_all

      visit person_path(person)

      within("#person_medication_#{person_medication.id}") do
        click_button '💊 Take'
      end

      expect(page).to have_content('Medication taken successfully')
    end

    it 'disables the take button when max daily doses reached' do
      person_medication.update!(max_daily_doses: 2, min_hours_between_doses: nil)
      person_medication.medication_takes.delete_all
      2.times do
        MedicationTake.create!(
          person_medication: person_medication,
          taken_at: Time.current,
          amount_ml: 5
        )
      end

      visit person_path(person)

      within("#person_medication_#{person_medication.id}") do
        expect(page).to have_button('💊 Take', disabled: true)
      end
    end

    it 'disables the take button when minimum hours not passed' do
      person_medication.update!(max_daily_doses: nil, min_hours_between_doses: 6)
      person_medication.medication_takes.delete_all
      MedicationTake.create!(
        person_medication: person_medication,
        taken_at: 2.hours.ago,
        amount_ml: 5
      )

      visit person_path(person)

      within("#person_medication_#{person_medication.id}") do
        expect(page).to have_button('💊 Take', disabled: true)
      end
    end
  end

  describe 'viewing today\'s doses on card' do
    let(:person_medication) { person_medications(:john_vitamin_d) }

    let!(:take) do
      MedicationTake.create!(
        person_medication: person_medication,
        taken_at: Time.current,
        amount_ml: 5
      )
    end

    it 'displays today\'s doses on the medication card' do
      visit person_path(person)

      within("#person_medication_#{person_medication.id}") do
        expect(page).to have_content("TODAY'S DOSES")
        expect(page).to have_content(take.taken_at.strftime('%l:%M %p').strip)
        expect(page).to have_content('5ML')
      end
    end
  end
end
