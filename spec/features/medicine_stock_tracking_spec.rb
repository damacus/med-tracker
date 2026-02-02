# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medicine Stock Tracking', type: :system do
  fixtures :accounts, :people, :users, :medicines, :dosages, :prescriptions

  let(:admin) { users(:admin) }
  let(:person) { people(:john) }
  let(:medicine) { medicines(:paracetamol) }
  let(:prescription) { prescriptions(:john_paracetamol) }

  before do
    # Ensure medicine has known stock level
    medicine.update!(stock: 10, reorder_threshold: 5)

    # Login as admin using helper that clears 2FA
    login_as(admin)
  end

  it 'displays current stock on the dashboard' do
    visit root_path

    within "#prescription_#{prescription.id}" do
      expect(page).to have_content('In Stock')
      expect(page).to have_content('Qty: 10')
    end
  end

  it 'shows low stock badge when stock reaches threshold' do
    medicine.update!(stock: 5)
    visit root_path

    within "#prescription_#{prescription.id}" do
      expect(page).to have_content('Low Stock')
      expect(page).to have_content('Qty: 5')
    end
  end

  it 'shows out of stock badge when stock is zero' do
    medicine.update!(stock: 0)
    visit root_path

    within "#prescription_#{prescription.id}" do
      expect(page).to have_content('Out of Stock')
      expect(page).to have_content('Qty: 0')
    end
  end

  it 'deducts stock when taking a dose' do
    visit root_path

    within "#prescription_#{prescription.id}" do
      expect(page).to have_content('Qty: 10')

      # Use the specific test ID for the Take Now button
      click_button 'Take Now', match: :first
    end

    expect(page).to have_content(I18n.t('take_medicines.success'))

    within "#prescription_#{prescription.id}" do
      expect(page).to have_content('Qty: 9')
    end
  end
end
