# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Schedules workflow' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :carer_relationships

  let(:admin_user) { users(:admin) }

  before { sign_in(admin_user) }

  describe 'GET /schedules/workflow' do
    it 'renders onboarding fields for schedule creation' do
      get schedules_workflow_path

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('Type (OTC or prescribed)')
      expect(response.body).to include('Name of med')
      expect(response.body).to include('Person name')
      expect(response.body).to include('Dose, frequency')
      expect(response.body).to include('Schedule (break this down)')
    end

    it 'preselects medication from query params' do
      get schedules_workflow_path, params: { medication_id: medications(:paracetamol).id }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include("<option value=\"#{medications(:paracetamol).id}\" selected>Paracetamol</option>")
    end
  end

  describe 'POST /schedules/workflow' do
    it 'redirects to prefilled person schedule form' do
      post start_schedules_workflow_path, params: {
        person_id: people(:john).id,
        medication_id: medications(:paracetamol).id,
        schedule_type: 'prescribed',
        frequency: 'Twice daily'
      }

      expect(response).to redirect_to(
        new_person_schedule_path(
          people(:john),
          medication_id: medications(:paracetamol).id,
          schedule_type: 'prescribed',
          frequency: 'Twice daily'
        )
      )
    end
  end

  describe 'GET /people/:person_id/schedules/new' do
    it 'prefills selected medication and frequency from workflow params' do
      get new_person_schedule_path(people(:john)), params: {
        medication_id: medications(:paracetamol).id,
        frequency: 'Twice daily'
      }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include("value=\"#{medications(:paracetamol).id}\"")
      expect(response.body).to include('Twice daily')
    end
  end
end
