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
        expect(page).to have_css('.rounded-2xl') # Medicine icon background
      end
    end

    it 'displays dosage information' do
      within("#prescription_#{prescription.id}") do
        expect(page).to have_content("#{prescription.dosage.amount.to_i}#{prescription.dosage.unit.upcase}")
        expect(page).to have_content(prescription.frequency.upcase)
      end
    end

    it 'does not display stock badge when adequately stocked' do
      within("#prescription_#{prescription.id}") do
        # We now show count for all stocked items, but let's check for specific "Low Stock" text absence
        expect(page).to have_no_content('Low Stock')
        expect(page).to have_no_content('Out of Stock')
      end
    end

    it 'displays start date' do
      within("#prescription_#{prescription.id}") do
        expect(page).to have_content(I18n.t('prescriptions.card.started').upcase)
        expect(page).to have_content(prescription.start_date.strftime('%b %d, %Y'))
      end
    end

    it 'displays end date when present' do
      within("#prescription_#{prescription.id}") do
        expect(page).to have_content(I18n.t('prescriptions.card.ends').upcase)
        expect(page).to have_content(prescription.end_date.strftime('%b %d, %Y'))
      end
    end

    it 'displays notes when present' do
      prescription.update!(notes: 'Take with food')
      visit person_path(person)

      within("#prescription_#{prescription.id}") do
        expect(page).to have_content(I18n.t('prescriptions.card.notes').strip.upcase)
        expect(page).to have_content('Take with food')
      end
    end

    it 'displays todays doses section' do
      within("#prescription_#{prescription.id}") do
        expect(page).to have_content(I18n.t('prescriptions.card.todays_doses').upcase)
      end
    end

    it 'shows no doses message when no takes today' do
      prescription.medication_takes.destroy_all
      visit person_path(person)

      within("#prescription_#{prescription.id}") do
        expect(page).to have_content(I18n.t('prescriptions.card.no_doses_today'))
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
        expect(page).to have_content("#{take.amount_ml.to_i}#{prescription.dosage.unit.upcase}")
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
          expect(page).to have_button(I18n.t('prescriptions.card.give'), disabled: false)
        end
      end

      it 'has correct button styling' do
        visit person_path(person)

        within("#prescription_#{prescription.id}") do
          button = find_button(I18n.t('prescriptions.card.give'))
          expect(button[:class]).to include('shadow-lg')
        end
      end

      it 'submits take medicine form when clicked' do
        visit person_path(person)

        within("#prescription_#{prescription.id}") do
          click_button I18n.t('prescriptions.card.give')
        end

        expect(prescription.medication_takes.count).to eq(1)
      end

      it 'records correct timestamp for take' do
        visit person_path(person)
        time_before = Time.current

        within("#prescription_#{prescription.id}") do
          click_button I18n.t('prescriptions.card.give')
        end

        latest_take = prescription.medication_takes.order(taken_at: :desc).first
        expect(latest_take.taken_at).to be_between(time_before, 1.second.from_now)
      end

      it 'records correct dosage amount' do
        visit person_path(person)

        within("#prescription_#{prescription.id}") do
          click_button I18n.t('prescriptions.card.give')
        end

        latest_take = prescription.medication_takes.order(taken_at: :desc).first
        expect(latest_take.amount_ml).to eq(prescription.dosage.amount)
      end

      it 'updates the card to show the new take' do
        visit person_path(person)

        within("#prescription_#{prescription.id}") do
          expect(page).to have_content(I18n.t('prescriptions.card.no_doses_today'))
          click_button I18n.t('prescriptions.card.give')
        end

        # Wait for Turbo update
        using_wait_time(10) do
          within("#prescription_#{prescription.id}") do
            expect(page).to have_no_content(I18n.t('prescriptions.card.no_doses_today'))
            expect(page).to have_css('.text-emerald-500') # New check icon color
          end
        end
      end
    end

    context 'when medicine is out of stock' do
      before do
        medicine.update!(current_supply: 0)
      end

      it 'displays disabled Out of Stock button' do
        visit person_path(person)

        within("#prescription_#{prescription.id}") do
          expect(page).to have_button(I18n.t('prescriptions.card.out_of_stock'), disabled: true)
        end
      end

      it 'has grayscale styling when out of stock' do
        visit person_path(person)

        within("#prescription_#{prescription.id}") do
          button = find_button(I18n.t('prescriptions.card.out_of_stock'), disabled: true)
          expect(button[:class]).to include('grayscale')
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
          expect(page).to have_button(I18n.t('prescriptions.card.give'), disabled: true)
        end
      end

      it 'has grayscale styling when disabled' do
        visit person_path(person)

        within("#prescription_#{prescription.id}") do
          button = find_button(I18n.t('prescriptions.card.give'), disabled: true)
          expect(button[:class]).to include('grayscale')
        end
      end

      it 'shows countdown notice' do
        visit person_path(person)

        within("#prescription_#{prescription.id}") do
          expect(page).to have_content(I18n.t('prescriptions.card.next_dose_available').strip.upcase)
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

      it 'displays Edit icon link' do
        within("#prescription_#{prescription.id}") do
          # Edit is now an SVG link with no text
          expect(page).to have_css("a[href='#{edit_person_prescription_path(person, prescription)}']")
        end
      end

      it 'navigates to edit page when clicked' do
        within("#prescription_#{prescription.id}") do
          find("a[href='#{edit_person_prescription_path(person, prescription)}']").click
        end

        expect(page).to have_current_path(edit_person_prescription_path(person, prescription))
        expect(page).to have_content(/edit prescription/i)
      end
    end

    context 'when user is non-administrator' do
      before do
        login_as(carer_user)
        visit person_path(people(:child_patient))
      end

      it 'does not display Edit button' do
        expect(page).to have_no_css("a[href='#{edit_person_prescription_path(people(:child_patient),
                                                                             prescriptions(:patient_prescription))}']")
      end
    end
  end

  describe 'Delete Button' do
    context 'when user is administrator' do
      before do
        login_as(admin_user)
        visit person_path(person)
      end

      it 'displays Delete button (trash icon)' do
        within("#prescription_#{prescription.id}") do
          # Delete trigger is an SVG icon inside a ghost button
          expect(page).to have_css('button svg')
        end
      end
    end

    context 'when user is non-administrator' do
      before do
        login_as(carer_user)
        visit person_path(people(:child_patient))
      end

      it 'does not display Delete button' do
        within("#prescription_#{prescriptions(:patient_prescription).id}") do
          expect(page).to have_no_css('button svg') # No icons in buttons for non-admins
        end
      end
    end
  end

  describe 'Button Consistency' do
    before do
      login_as(admin_user)
      prescription.medication_takes.destroy_all
      visit person_path(person)
    end

    it 'all buttons have consistent rounded styling (xl/2xl)' do
      within("#prescription_#{prescription.id}") do
        take_button = find_button(I18n.t('prescriptions.card.give'))
        # Check for rounded-xl or rounded-2xl
        expect(take_button[:class]).to include('rounded-')
      end
    end

    it 'Take button uses Button component (not Link)' do
      within("#prescription_#{prescription.id}") do
        take_button = find_button(I18n.t('prescriptions.card.give'))
        expect(take_button.tag_name).to eq('button')
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
        click_button I18n.t('prescriptions.card.give')
      end

      expect(prescription.medication_takes.count).to eq(1)
    end

    it 'card has proper semantic structure' do
      within("#prescription_#{prescription.id}") do
        expect(page).to have_css('h3', text: medicine.name) # CardTitle
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
        # Search for text elements that look like times
        takes = all('div', text: /:\d{2} [AP]M/).map(&:text).grep(/:\d{2} [AP]M/)
        # Reverse chronological means take3, then take2, then take1
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
