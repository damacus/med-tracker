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

    it 'filters dashboard records to the default selected person' do
      child_medication = create(:medication, name: 'Child only dashboard medicine')
      create(:schedule, person: people(:child_patient), medication: child_medication)
      jane_medication = create(:medication, name: 'Jane only dashboard medicine')
      create(:schedule, person: people(:jane), medication: jane_medication)
      sign_in(users(:jane))

      get root_path

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('Jane only dashboard medicine')
      expect(response.body).not_to include('Child only dashboard medicine')
    end

    it 'shows the selected person records when a dashboard person is requested' do
      child_medication = create(:medication, name: 'Child selected dashboard medicine')
      create(:schedule, person: people(:child_patient), medication: child_medication)
      jane_medication = create(:medication, name: 'Jane hidden dashboard medicine')
      create(:schedule, person: people(:jane), medication: jane_medication)
      sign_in(users(:jane))

      get root_path, params: { dashboard_person_id: people(:child_patient).id }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('Child selected dashboard medicine')
      expect(response.body).not_to include('Jane hidden dashboard medicine')
    end

    it 'keeps all-family behavior without persisting it across dashboard visits' do
      child_medication = create(:medication, name: 'Child family dashboard medicine')
      create(:schedule, person: people(:child_patient), medication: child_medication)
      jane_medication = create(:medication, name: 'Jane family dashboard medicine')
      create(:schedule, person: people(:jane), medication: jane_medication)
      sign_in(users(:jane))

      get root_path, params: { dashboard_person_id: 'all' }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('Child family dashboard medicine')
      expect(response.body).to include('Jane family dashboard medicine')

      get root_path

      expect(response.body).to include('Jane family dashboard medicine')
      expect(response.body).not_to include('Child family dashboard medicine')
    end
  end
end
