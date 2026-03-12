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

    expect(page).to have_content('Experiments')
    expect(page).to have_content('Add Medication Wizard Style')
    expect(page).to have_content('Full page')
    expect(page).to have_content('Modal')
    expect(page).to have_content('Slide-over')
  end

  it 'defaults to fullpage variant' do
    visit profile_path

    expect(page).to have_field('user[wizard_variant]', with: 'fullpage', checked: true)
  end
end
