# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Profile Experiments' do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:user) { users(:admin) }

  before do
    driven_by(:rack_test)
    sign_in(user)
  end

  it 'shows the Experiments card with wizard style options' do
    visit profile_path

    expect(page).to have_text('Experiments')
    expect(page).to have_text('Add Medication Wizard Style')
    expect(page).to have_text('Full page')
    expect(page).to have_text('Modal')
    expect(page).to have_text('Slide-over')
    expect(page).to have_text('Dashboard layout')
    expect(page).to have_text('Current dashboard')
    expect(page).to have_text('Time-first')
    expect(page).to have_text('Family lanes')
    expect(page).to have_text('Calm focus')
  end

  it 'defaults to fullpage variant' do
    visit profile_path

    expect(page).to have_field('account[wizard_variant]', with: 'fullpage', checked: true)
    expect(page).to have_field('account[dashboard_variant]', with: 'current', checked: true)
  end
end
