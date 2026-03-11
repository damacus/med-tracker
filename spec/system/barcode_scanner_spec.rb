# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'BarcodeScanner' do
  fixtures :accounts, :people, :users

  let(:user) { users(:john) }

  context 'with static HTML (rack_test)' do
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

      scanner_region = find_by_id('barcode-scanner-region', visible: false)
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

  context 'with JavaScript (Playwright)' do
    before do
      driven_by(:playwright)
      login_as(user)
    end

    it 'loads html5-qrcode and transitions state when Start Scanner is clicked' do
      visit medication_finder_path

      scanner = find('[data-testid="barcode-scanner"]')
      expect(scanner['data-scanner-state']).to eq('idle')

      click_button 'Start Scanner'

      using_wait_time(10) do
        expect(page).to have_text('Requesting camera access').or(
          have_text('Point your camera at a barcode')
        ).or(
          have_text('Camera access was denied')
        ).or(
          have_text('Scanner error')
        )
      end
    end

    it 'shows Start Scanner button again after error recovery' do
      visit medication_finder_path

      click_button 'Start Scanner'

      using_wait_time(10) do
        expect(page).to have_text('Scanner error').or(have_text('Camera access was denied'))
      end

      expect(page).to have_button('Start Scanner')
    end
  end
end
