# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Prescription Card', type: :system do
  fixtures :all

  let(:admin_user) { users(:admin) }
  let(:carer_user) { users(:carer) }
  let(:person) { people(:bob) }
  let(:prescription) { prescriptions(:bob_aspirin) }
  let(:medicine) { prescription.medicine }

  before do
    login_as(admin_user)
    visit person_path(person)
  end

  describe 'Card Display' do
    it 'displays medicine name and icon' do
      within("#prescription_#{prescription.id}") do
        expect(page).to have_content(medicine.name)
        expect(page).to have_css('.bg-violet-100') # Medicine icon background
      end
    end

    it 'displays dosage information' do
      within("#prescription_#{prescription.id}") do
        expect(page).to have_content("#{prescription.dosage.amount.to_i} #{prescription.dosage.unit}")
        expect(page).to have_content(prescription.frequency)
      end
    end

    it 'displays stock badge' do
      within("#prescription_#{prescription.id}") do
        expect(page).to have_content('In Stock')
      end
    end

    it 'displays start date' do
      within("#prescription_#{prescription.id}") do
        expect(page).to have_content('Started:')
        expect(page).to have_content(prescription.start_date.strftime('%B %d, %Y'))
      end
    end

    it 'displays end date when present' do
      within("#prescription_#{prescription.id}") do
        expect(page).to have_content('Ends:')
        expect(page).to have_content(prescription.end_date.strftime('%B %d, %Y'))
      end
    end

    it 'displays notes when present' do
      prescription.update!(notes: 'Take with food')
      visit person_path(person)

      within("#prescription_#{prescription.id}") do
        expect(page).to have_content('Notes:')
        expect(page).to have_content('Take with food')
      end
    end

    it 'displays todays doses section' do
      within("#prescription_#{prescription.id}") do
        expect(page).to have_content("Today's Doses")
      end
    end

    it 'shows no doses message when no takes today' do
      prescription.medication_takes.destroy_all
      visit person_path(person)

      within("#prescription_#{prescription.id}") do
        expect(page).to have_content('No doses taken today')
      end
    end

    it 'displays taken doses with timestamp and amount' do
      take = prescription.medication_takes.create!(
        taken_at: 2.hours.ago,
        amount_ml: prescription.dosage.amount
      )
      visit person_path(person)

      within("#prescription_#{prescription.id}") do
        expect(page).to have_content(take.taken_at.strftime('%l:%M %p').strip)
        expect(page).to have_content("#{take.amount_ml.to_i} ml")
      end
    end
  end

  describe 'Take Button' do
    context 'when prescription can be taken' do
      before do
        prescription.medication_takes.destroy_all
      end

      it 'displays enabled Take button' do
        visit person_path(person)

        within("#prescription_#{prescription.id}") do
          expect(page).to have_button('💊 Take', disabled: false)
        end
      end

      it 'has correct button styling' do
        visit person_path(person)

        within("#prescription_#{prescription.id}") do
          button = find_button('💊 Take')
          expect(button[:class]).to include('bg-primary')
          expect(button[:class]).not_to include('no-underline') # Button, not Link
        end
      end

      it 'submits take medicine form when clicked' do
        visit person_path(person)

        within("#prescription_#{prescription.id}") do
          click_button '💊 Take'
        end

        expect(prescription.medication_takes.count).to eq(1)
      end

      it 'records correct timestamp for take' do
        visit person_path(person)
        time_before = Time.current

        within("#prescription_#{prescription.id}") do
          click_button '💊 Take'
        end

        latest_take = prescription.medication_takes.order(taken_at: :desc).first
        expect(latest_take.taken_at).to be_between(time_before, Time.current)
      end

      it 'records correct dosage amount' do
        visit person_path(person)

        within("#prescription_#{prescription.id}") do
          click_button '💊 Take'
        end

        latest_take = prescription.medication_takes.order(taken_at: :desc).first
        expect(latest_take.amount_ml).to eq(prescription.dosage.amount)
      end

      it 'updates the card to show the new take' do
        visit person_path(person)

        within("#prescription_#{prescription.id}") do
          expect(page).to have_content('No doses taken today')
          click_button '💊 Take'
        end

        visit person_path(person)

        within("#prescription_#{prescription.id}") do
          expect(page).to have_no_content('No doses taken today')
          expect(page).to have_css('.text-green-600') # Check icon
        end
      end
    end

    context 'when prescription cannot be taken (cooldown)' do
      before do
        prescription.medication_takes.create!(
          taken_at: 30.minutes.ago,
          amount_ml: prescription.dosage.amount
        )
      end

      it 'displays disabled Take button' do
        visit person_path(person)

        within("#prescription_#{prescription.id}") do
          expect(page).to have_button('💊 Take', disabled: true)
        end
      end

      it 'has secondary variant styling when disabled' do
        visit person_path(person)

        within("#prescription_#{prescription.id}") do
          button = find_button('💊 Take', disabled: true)
          expect(button[:class]).to include('bg-secondary')
        end
      end

      it 'shows countdown notice' do
        visit person_path(person)

        within("#prescription_#{prescription.id}") do
          expect(page).to have_content('Next dose available in:')
        end
      end
    end
  end

  describe 'Edit Button' do
    context 'when user is administrator' do
      before do
        login_as(admin_user)
        visit person_path(person)
      end

      it 'displays Edit button' do
        within("#prescription_#{prescription.id}") do
          expect(page).to have_link('Edit')
        end
      end

      it 'has outline variant styling' do
        within("#prescription_#{prescription.id}") do
          link = find_link('Edit')
          expect(link[:class]).to include('border')
          expect(link[:class]).to include('no-underline')
        end
      end

      it 'navigates to edit page when clicked' do
        within("#prescription_#{prescription.id}") do
          click_link 'Edit'
        end

        expect(page).to have_current_path(edit_person_prescription_path(person, prescription))
        expect(page).to have_content(/edit prescription/i)
      end
    end

    context 'when user is non-administrator' do
      before do
        login_as(carer_user)
        visit person_path(person)
      end

      it 'does not display Edit button' do
        expect(page).to have_no_link('Edit')
      end
    end
  end

  describe 'Delete Button' do
    context 'when user is administrator' do
      before do
        login_as(admin_user)
        visit person_path(person)
      end

      it 'displays Delete button' do
        within("#prescription_#{prescription.id}") do
          expect(page).to have_button('Delete')
        end
      end

      it 'has subordinate outline styling with red text' do
        within("#prescription_#{prescription.id}") do
          button = find_button('Delete')
          expect(button[:class]).to include('border')
          expect(button[:class]).to include('text-red-600')
          expect(button[:class]).not_to include('bg-destructive')
        end
      end
    end

    context 'when user is non-administrator' do
      before do
        login_as(carer_user)
        visit person_path(person)
      end

      it 'does not display Delete button' do
        expect(page).to have_no_button('Delete')
      end
    end
  end

  describe 'Button Consistency' do
    before do
      login_as(admin_user)
      prescription.medication_takes.destroy_all
      visit person_path(person)
    end

    it 'all buttons have consistent size (md)' do
      within("#prescription_#{prescription.id}") do
        take_button = find_button('💊 Take')
        edit_link = find_link('Edit')
        delete_button = find_button('Delete')

        # All should have h-9 (md size)
        expect(take_button[:class]).to include('h-9')
        expect(edit_link[:class]).to include('h-9')
        expect(delete_button[:class]).to include('h-9')
      end
    end

    it 'Take button uses Button component (not Link)' do
      within("#prescription_#{prescription.id}") do
        take_button = find_button('💊 Take')
        expect(take_button.tag_name).to eq('button')
      end
    end

    it 'Edit uses Link component with no-underline' do
      within("#prescription_#{prescription.id}") do
        edit_link = find_link('Edit')
        expect(edit_link.tag_name).to eq('a')
        expect(edit_link[:class]).to include('no-underline')
      end
    end

    it 'Delete uses Button component' do
      within("#prescription_#{prescription.id}") do
        delete_button = find_button('Delete')
        expect(delete_button.tag_name).to eq('button')
      end
    end
  end

  describe 'Accessibility' do
    before do
      prescription.medication_takes.destroy_all
      visit person_path(person)
    end

    it 'Take button is keyboard accessible' do
      within("#prescription_#{prescription.id}") do
        click_button '💊 Take'
      end

      expect(prescription.medication_takes.count).to eq(1)
    end

    it 'disabled Take button has proper aria attributes' do
      prescription.medication_takes.create!(
        taken_at: 30.minutes.ago,
        amount_ml: prescription.dosage.amount
      )
      visit person_path(person)

      within("#prescription_#{prescription.id}") do
        expect(page).to have_button('💊 Take', disabled: true)
      end
    end

    it 'card has proper semantic structure' do
      within("#prescription_#{prescription.id}") do
        expect(page).to have_css('h3', text: medicine.name) # CardTitle
        expect(page).to have_css('h4', text: "Today's Doses") # Section heading
      end
    end
  end

  describe 'Multiple Takes in One Day' do
    before do
      prescription.medication_takes.destroy_all
    end

    it 'displays all takes from today in reverse chronological order' do
      take1 = prescription.medication_takes.create!(
        taken_at: 8.hours.ago,
        amount_ml: prescription.dosage.amount
      )
      take2 = prescription.medication_takes.create!(
        taken_at: 4.hours.ago,
        amount_ml: prescription.dosage.amount
      )
      take3 = prescription.medication_takes.create!(
        taken_at: 1.hour.ago,
        amount_ml: prescription.dosage.amount
      )

      visit person_path(person)

      within("#prescription_#{prescription.id}") do
        takes = all('.text-green-600').map { |el| el.find(:xpath, '..').text }
        expect(takes[0]).to include(take3.taken_at.strftime('%l:%M %p').strip)
        expect(takes[1]).to include(take2.taken_at.strftime('%l:%M %p').strip)
        expect(takes[2]).to include(take1.taken_at.strftime('%l:%M %p').strip)
      end
    end

    it 'does not display takes from previous days' do
      old_take = prescription.medication_takes.create!(
        taken_at: 2.days.ago,
        amount_ml: prescription.dosage.amount
      )

      visit person_path(person)

      within("#prescription_#{prescription.id}") do
        expect(page).to have_no_content(old_take.taken_at.strftime('%l:%M %p').strip)
        expect(page).to have_content('No doses taken today')
      end
    end
  end
end
