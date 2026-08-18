# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Mobile create actions' do
  fixtures :all

  let(:user) { users(:admin) }

  before do
    sign_in(user)
    page.current_window.resize_to(390, 844)
  end

  it 'shows allowed create actions on mobile', :browser do
    visit people_path

    expect(page).to have_link('New Person', href: new_person_path)

    visit locations_path

    expect(page).to have_link('Add Location', href: new_location_path)
  end
end
