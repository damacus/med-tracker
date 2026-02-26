# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'PersonMedicationsController medication options' do
  fixtures :accounts, :people, :locations, :medications, :users, :carer_relationships

  let(:adult_patient_user) { users(:adult_patient) }
  let(:adult_patient_person) { people(:adult_patient_person) }
  let(:admin_user) { users(:admin) }
  let(:admin_person) { people(:admin) }

  describe 'GET /people/:person_id/person_medications/new' do
    context 'when user can create person medications for themselves' do
      before { sign_in(adult_patient_user) }

      it 'shows all medications as options' do
        get new_person_person_medication_path(adult_patient_person)

        expect(response).to have_http_status(:ok)
        Medication.find_each do |medication|
          expect(response.body).to include(medication.name)
        end
      end
    end

    context 'when user is an administrator' do
      before { sign_in(admin_user) }

      it 'shows all medications' do
        get new_person_person_medication_path(admin_person)

        expect(response).to have_http_status(:ok)
        Medication.find_each do |medication|
          expect(response.body).to include(medication.name)
        end
      end
    end
  end

  describe 'POST /people/:person_id/person_medications' do
    context 'when user can create person medications and validation fails' do
      before { sign_in(adult_patient_user) }

      it 'shows all medications again on re-render' do
        post person_person_medications_path(adult_patient_person),
             params: { person_medication: { medication_id: '', notes: '' } }

        expect(response).to have_http_status(:unprocessable_content)
        Medication.find_each do |medication|
          expect(response.body).to include(medication.name)
        end
      end
    end
  end
end
