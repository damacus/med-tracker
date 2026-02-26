# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'MedicationFinder' do
  fixtures :accounts, :people, :users

  let(:user) { users(:john) }

  before do
    driven_by(:rack_test)
    login_as(user)
  end

  it 'displays the medication finder page' do
    visit medication_finder_path

    within '[data-testid="medication-finder"]' do
      aggregate_failures 'medication finder content' do
        expect(page).to have_content('Medication Finder')
        expect(page).to have_field('medication-search-input')
        expect(page).to have_button('Search')
        expected_text = 'Search the NHS Dictionary of Medications and Devices (dm+d) by name or active ingredient.'
        expect(page).to have_content(expected_text)
      end
    end
  end
end
