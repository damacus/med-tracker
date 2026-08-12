# frozen_string_literal: true

require 'rails_helper'

# This system test verifies the main site navigation using the Capybara DSL.
RSpec.describe 'Navigation', :browser do
  fixtures :accounts, :people, :users

  before do
    driven_by(:playwright)
  end

  context 'when user is not authenticated' do
    it 'keeps account verification in the auth shell with a login return' do
      page.current_window.resize_to(375, 667)
      visit '/verify-account-resend'

      expect(page).to have_field('Email address')
      expect(page).to have_link('Welcome back', href: '/login')
      expect(page).to have_no_css('[data-responsive-shell-role="sidebar"]')
      expect(page).to have_no_css('[data-testid="mobile-rail"]')
      expect(page).to have_no_css('nav.nav')
    end
  end
end
