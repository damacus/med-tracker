# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Dashboard' do
  fixtures :accounts, :users, :medicines, :dosages, :prescriptions, :people, :carer_relationships, :medication_takes

  it 'loads the dashboard for a signed-in user and shows family-wide doses' do
    sign_in(users(:jane))

    visit dashboard_path

    expect(page).to have_content('Family Dashboard')
    
    # Jane's medication
    expect(page).to have_content('Ibuprofen')
    expect(page).to have_content('Jane Doe')

    # Child's medication
    expect(page).to have_content('Child Patient')
  end

  it 'allows taking a dose directly from the dashboard' do
    sign_in(users(:jane))
    visit dashboard_path

    # Debug: print statuses found
    puts "Statuses found: #{all('.mb-4').map(&:text).join(' | ')}"

    # Find an upcoming dose
    within find('.mb-4', text: 'Upcoming', match: :first) do
      click_on '💊 Take'
    end

    expect(page).to have_content('Medicine taken successfully', wait: 10)
  end
end
