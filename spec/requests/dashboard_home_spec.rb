# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Dashboard home rendering' do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules,
           :carer_relationships, :person_medications, :medication_takes

  describe 'GET /' do
    it 'renders the dashboard with Add Medication and hides Add Person for admins' do
      sign_in(users(:damacus))

      get root_path

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('Dashboard')
      expect(response.body).to include('Add Medication')
      expect(response.body).not_to include('Add Person')
    end

    it 'renders the dashboard with Add Person for parents' do
      sign_in(users(:jane))

      get root_path

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('Dashboard')
      expect(response.body).to include('Add Person')
    end
  end
end
