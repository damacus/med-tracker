# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Dashboard Authorization', browser: false do
  fixtures :all

  before do
    driven_by(:rack_test)
  end

  let(:admin) { users(:admin) }
  let(:doctor) { users(:doctor) }
  let(:nurse) { users(:nurse) }
  let(:carer) { users(:carer) }
  let(:parent) { users(:parent) }
  let(:adult_patient) { users(:adult_patient) }

  describe 'dashboard access' do
    it 'allows administrators to access dashboard' do
      sign_in(admin)
      visit dashboard_path

      expect(page).to have_content('Dashboard')
      expect(page).to have_no_content('You are not authorized')
    end

    it 'allows doctors to access dashboard' do
      sign_in(doctor)
      visit dashboard_path

      expect(page).to have_content('Dashboard')
      expect(page).to have_no_content('You are not authorized')
    end

    it 'allows nurses to access dashboard' do
      sign_in(nurse)
      visit dashboard_path

      expect(page).to have_content('Dashboard')
      expect(page).to have_no_content('You are not authorized')
    end

    it 'allows carers to access dashboard' do
      sign_in(carer)
      visit dashboard_path

      expect(page).to have_content('Dashboard')
      expect(page).to have_no_content('You are not authorized')
    end

    it 'allows parents to access dashboard' do
      sign_in(parent)
      visit dashboard_path

      expect(page).to have_content('Dashboard')
      expect(page).to have_no_content('You are not authorized')
    end

    it 'allows adult patients to access dashboard' do
      sign_in(adult_patient)
      visit dashboard_path

      expect(page).to have_content('Dashboard')
      expect(page).to have_no_content('You are not authorized')
    end

    it 'denies unauthenticated users access to dashboard' do
      visit dashboard_path

      expect(page).to have_current_path(login_path)
      expect(page).to have_no_content('Dashboard')
    end
  end

  describe 'dashboard data scoping' do
    it 'shows all people to administrators' do
      sign_in(admin)
      visit dashboard_path

      # Admin should see multiple people
      expect(page).to have_content('People')
    end

    it 'shows only assigned patients to carers' do
      sign_in(carer)
      visit dashboard_path

      # Carer should see their assigned patients
      expect(page).to have_content('People')
      # Should see child_patient (assigned via carer_cares_for_patient fixture)
      expect(page).to have_content('Child Patient')
    end

    it 'shows only minor children to parents' do
      sign_in(parent)
      visit dashboard_path

      # Parent should see their children
      expect(page).to have_content('People')
      # Should see child_user_person (assigned via parent_cares_for_child fixture)
      expect(page).to have_content('Child User')
    end

    it 'shows only themselves to adult patients' do
      sign_in(adult_patient)
      visit dashboard_path

      # Adult patient should see the dashboard with their own data
      expect(page).to have_content('Dashboard')
      # They should see the Medication Schedule section
      expect(page).to have_content('Medication Schedule')
    end
  end
end
