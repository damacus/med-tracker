# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'BarcodeScanner' do
  fixtures :accounts, :people, :users

  let(:user) { users(:john) }

  before do
    driven_by(:rack_test)
    login_as(user)
  end

  it 'displays the barcode scanner on the medication finder page' do
    visit medication_finder_path

    within '[data-testid="barcode-scanner"]' do
      aggregate_failures 'scanner component elements' do
        expect(page).to have_button('Start Scanner')
        expect(page).to have_button('Stop Scanner', visible: :hidden)
        expect(page).to have_content('Or enter barcode manually')
        expect(page).to have_field('manual-barcode')
        expect(page).to have_button('Submit')
      end
    end
  end

  it 'renders the scanner region as hidden initially' do
    visit medication_finder_path

    scanner_region = find('#barcode-scanner-region', visible: false)
    expect(scanner_region).not_to be_visible
  end

  it 'provides manual barcode input as a fallback' do
    visit medication_finder_path

    within '[data-testid="manual-barcode-input"]' do
      expect(page).to have_field('manual-barcode', type: 'text')
      expect(page).to have_button('Submit')
    end
  end
end
