# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Dashboard' do
  fixtures :users, :medicines, :dosages, :prescriptions

  it 'loads the dashboard for a signed-in user' do
    sign_in(users(:john))

    visit dashboard_path

    within '[data-testid="dashboard"]' do
      aggregate_failures 'dashboard content' do
        expect(page).to have_content('Medicine Tracker Dashboard')
        expect(page).to have_link('Add Medicine', href: new_medicine_path)
        expect(page).to have_link('Add Person', href: new_user_path)
      end
    end
  end
end
