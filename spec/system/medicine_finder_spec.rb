# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'MedicineFinder' do
  fixtures :users

  let(:user) { users(:john) }

  before do
    driven_by(:rack_test)
    login_as(user)
  end

  it 'displays the medicine finder page' do
    visit medicine_finder_path

    within '[data-testid="medicine-finder"]' do
      aggregate_failures 'medicine finder content' do
        expect(page).to have_content('Medicine Finder')
        expect(page).to have_field('medicine-search-input')
        expect(page).to have_button('Search')
        expect(page).to have_content('Search for medicines by name or active ingredient')
      end
    end
  end
end
