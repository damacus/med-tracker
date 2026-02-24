# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medicine Stock Tracking', type: :system do
  fixtures :accounts, :people, :users, :locations, :medicines, :dosages, :prescriptions

  let(:admin) { users(:admin) }
  let(:person) { people(:john) }
  let(:medicine) { medicines(:paracetamol) }
  let(:prescription) { prescriptions(:john_paracetamol) }

  before do
    # Ensure medicine has known stock level using current_supply
    medicine.update!(current_supply: 10, reorder_threshold: 5)

    # Clear any fixture medication_takes to avoid cooldown interference
    prescription.medication_takes.delete_all

    # Login as admin using helper that clears 2FA
    login_as(admin)
  end

  it 'displays current stock on the dashboard' do
    visit person_path(person)

    within "#prescription_#{prescription.id}" do
      # Badge doesn't show when adequately stocked (only for low/out of stock)
      expect(page).to have_no_content('In Stock')
      expect(page).to have_content('10')
    end
  end

  it 'shows low stock badge when stock reaches threshold' do
    medicine.update!(current_supply: 5)
    visit person_path(person)

    within "#prescription_#{prescription.id}" do
      expect(page).to have_content('Low Stock')
      expect(page).to have_content('5')
    end
  end

  it 'shows out of stock badge when stock is zero' do
    medicine.update!(current_supply: 0)
    visit person_path(person)

    within "#prescription_#{prescription.id}" do
      expect(page).to have_content('Out of Stock')
      expect(page).to have_content('0')
    end
  end

  it 'disables the Take Now button when out of stock' do
    medicine.update!(current_supply: 0)
    visit person_path(person)

    within "#prescription_#{prescription.id}" do
      expect(page).to have_css("[data-testid='take-prescription-#{prescription.id}-disabled']",
                               text: /Out of Stock/i)
    end
  end

  it 'deducts stock when taking a dose' do
    visit person_path(person)

    within "#prescription_#{prescription.id}" do
      expect(page).to have_content('10')

      # Use the specific test ID for the Take button
      find("[data-testid='take-prescription-#{prescription.id}']").click
    end

    expect(page).to have_content(/taken successfully/)

    within "#prescription_#{prescription.id}" do
      expect(page).to have_content('9 left')
    end
  end
end
