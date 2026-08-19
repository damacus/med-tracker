# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Profile Experiments', :js do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:user) { users(:admin) }

  before do
    sign_in(user)
  end

  it 'shows the Experiments card with wizard style options' do
    visit profile_path(section: 'advanced')
    find_by_id('profile-experiments-trigger').click

    expect(page).to have_text('Experiments')
    expect(page).to have_text('ADD MEDICATION WIZARD STYLE')
    expect(page).to have_text('Full page')
    expect(page).to have_text('Modal')
    expect(page).to have_text('Slide-over')
    expect(page).to have_text('DASHBOARD LAYOUT')
    expect(page).to have_text('Current dashboard')
    expect(page).to have_text('Time-first')
    expect(page).to have_text('Family lanes')
    expect(page).to have_text('Calm focus')
    expect(page).to have_text('ADD MEDICATION LAUNCHER')
    expect(page).to have_text('Current launcher')
    expect(page).to have_text('Context-aware')
  end

  it 'defaults to fullpage variant' do
    visit profile_path(section: 'advanced')
    find_by_id('profile-experiments-trigger').click

    expect(page).to have_field('account[wizard_variant]', with: 'fullpage', checked: true)
    expect(page).to have_field('account[dashboard_variant]', with: 'current', checked: true)
    expect(page).to have_field('account[medication_launcher_variant]', with: 'current', checked: true)
  end
end
