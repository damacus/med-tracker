# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'MedicineFinder' do
  fixtures :accounts, :people, :users

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
        expected_text = 'Search the NHS Dictionary of Medicines and Devices (dm+d) by name or active ingredient.'
        expect(page).to have_content(expected_text)
      end
    end
  end
end
