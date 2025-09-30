# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Person Medicines' do
  fixtures :people, :medicines, :users, :sessions

  let(:person) { people(:john) }
  let(:medicine) { medicines(:vitamin_d) }
  let(:user) { users(:john) }

  before do
    # Sign in
    visit login_path
    fill_in 'Email', with: user.email_address
    fill_in 'Password', with: 'password'
    click_button 'Sign in'
  end

  describe 'adding a non-prescription medicine' do
    it 'allows adding a vitamin/OTC medicine without a prescription' do
      visit person_path(person)

      # Should see a "My Medicines" section
      expect(page).to have_content('My Medicines')

      # Click to add a medicine
      click_link 'Add Medicine'

      # Fill in the form
      select medicine.name, from: 'Medicine'
      fill_in 'Notes', with: 'Take with breakfast'
      fill_in 'Max daily doses', with: '1'

      click_button 'Add Medicine'

      # Should see success message
      expect(page).to have_content('Medicine added successfully')

      # Should see the medicine in the list
      expect(page).to have_content(medicine.name)
      expect(page).to have_content('Take with breakfast')
    end
  end

  describe 'recording medicine takes' do
    let!(:person_medicine) do
      PersonMedicine.create!(
        person: person,
        medicine: medicine,
        max_daily_doses: 2,
        min_hours_between_doses: 6
      )
    end

    it 'allows recording a medicine take' do
      visit person_path(person)

      # Find the medicine card and click "Take"
      within("#person_medicine_#{person_medicine.id}") do
        click_button 'Take'
      end

      # Should see success message
      expect(page).to have_content('Medicine taken successfully')

      # Should see the take recorded
      within("#person_medicine_#{person_medicine.id}") do
        expect(page).to have_content("Today's Doses")
        expect(page).to have_content(Time.current.strftime('%l:%M %p').strip)
      end
    end

    it 'disables the take button when max daily doses reached' do
      # Create max doses for today
      2.times do
        MedicationTake.create!(
          person_medicine: person_medicine,
          taken_at: Time.current,
          amount_ml: 5
        )
      end

      visit person_path(person)

      # Button should be disabled
      within("#person_medicine_#{person_medicine.id}") do
        expect(page).to have_button('Take', disabled: true)
      end
    end

    it 'disables the take button when minimum hours not passed' do
      # Create a recent take
      MedicationTake.create!(
        person_medicine: person_medicine,
        taken_at: 2.hours.ago,
        amount_ml: 5
      )

      visit person_path(person)

      # Button should be disabled
      within("#person_medicine_#{person_medicine.id}") do
        expect(page).to have_button('Take', disabled: true)
      end
    end
  end

  describe 'viewing medication history' do
    let!(:person_medicine) do
      PersonMedicine.create!(
        person: person,
        medicine: medicine
      )
    end

    let!(:takes) do
      [
        MedicationTake.create!(
          person_medicine: person_medicine,
          taken_at: 2.days.ago,
          amount_ml: 5
        ),
        MedicationTake.create!(
          person_medicine: person_medicine,
          taken_at: 1.day.ago,
          amount_ml: 5
        ),
        MedicationTake.create!(
          person_medicine: person_medicine,
          taken_at: Time.current,
          amount_ml: 5
        )
      ]
    end

    it 'displays medication history in a table' do
      visit person_path(person)

      # Click to view history
      click_link 'View History'

      # Should see a table with all takes
      expect(page).to have_content('Medication History')

      within('table') do
        expect(page).to have_content(medicine.name)
        expect(page).to have_content('5 ml')

        # Should show dates
        takes.each do |take|
          expect(page).to have_content(take.taken_at.strftime('%B %d, %Y'))
        end
      end
    end
  end
end
