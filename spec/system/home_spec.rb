# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Home' do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules

  it 'loads the dashboard for an admin and hides Add Person' do
    sign_in(users(:damacus))

    visit root_path

    aggregate_failures 'dashboard content' do
      expect(page).to have_content('Dashboard')
      expect(page).to have_link('Add Medication', href: new_medication_path)
      expect(page).to have_no_link('Add Person')
    end
  end

  it 'loads the dashboard for a parent and shows Add Person' do
    sign_in(users(:jane))

    visit root_path

    aggregate_failures 'dashboard content' do
      expect(page).to have_content('Dashboard')
      expect(page).to have_link('Add Person', href: new_person_path)
    end
  end
end
