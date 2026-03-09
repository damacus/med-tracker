# frozen_string_literal: true

require 'rails_helper'

# This system test verifies the main site navigation using the Capybara DSL.
RSpec.describe 'Navigation' do
  fixtures :accounts, :people, :users

  before do
    driven_by(:playwright)
  end

  context 'when user is not authenticated' do
    it 'shows navigation with a login link' do
      page.current_window.resize_to(375, 667)
      visit root_path

      expect(page).to have_link('Login')
    end
  end
end
