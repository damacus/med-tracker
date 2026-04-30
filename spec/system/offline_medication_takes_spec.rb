# frozen_string_literal: true

require 'rails_helper'
require 'timeout'

RSpec.describe 'Offline medication takes', :js do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :schedules,
           :person_medications, :medication_takes

  let(:user) { users(:admin) }
  let(:medication) { medications(:gabapentin) }
  let!(:schedule) do
    Schedule.create!(
      person: people(:admin),
      medication: medication,
      dose_amount: 300,
      dose_unit: 'mg',
      frequency: 'As needed',
      start_date: Time.zone.today,
      end_date: 1.year.from_now.to_date,
      max_daily_doses: nil,
      min_hours_between_doses: nil
    )
  end

  it 'queues a take in the offline shell and syncs it when online fires' do
    medication.update!(current_supply: 1)

    login_as(user)
    visit offline_path

    expect(page).to have_content('OFFLINE CARE')
    expect(page).to have_content('Gabapentin')

    within('[data-testid="offline-dose-card"]', text: 'Gabapentin') do
      click_button 'Take now'
      expect(page).to have_content('QUEUED LOCALLY')
      expect(page).to have_button('Out of stock', disabled: true)
    end

    expect(page).to have_content('PENDING SYNC')
    expect(page).to have_css('[data-offline-shell-target="pendingCount"]', text: '1')

    page.execute_script('window.dispatchEvent(new Event("online"))')

    Timeout.timeout(5) do
      sleep 0.1 until MedicationTake.where(schedule: schedule).where.not(client_uuid: nil).exists?
    end

    expect(page).to have_css('[data-offline-shell-target="pendingCount"]', text: '0')
  end
end
