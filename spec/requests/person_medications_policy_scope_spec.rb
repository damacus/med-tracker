# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'PersonMedicationsController medication options' do
  fixtures :accounts, :people, :locations, :medications, :users, :carer_relationships

  let(:adult_patient_user) { users(:adult_patient) }
  let(:adult_patient_person) { people(:adult_patient_person) }
  let(:admin_user) { users(:admin) }
  let(:admin_person) { people(:admin) }
  let!(:foreign_medication) do
    Medication.create!(
      name: 'Foreign Household Medication',
      location: locations(:grandmas),
      category: 'Analgesic',
      dosage_amount: 250,
      dosage_unit: 'mg',
      current_supply: 10,
      reorder_threshold: 1
    )
  end

  describe 'GET /people/:person_id/person_medications/new' do
    context 'when user can create person medications for themselves' do
      before { sign_in(adult_patient_user) }

      it 'shows only accessible medications as options' do
        get new_person_person_medication_path(adult_patient_person)

        expect(response).to have_http_status(:ok)
        expect(response.body).to include(medications(:vitamin_d).name)
        expect(response.body).not_to include(foreign_medication.name)
      end
    end

    context 'when user is an administrator' do
      before { sign_in(admin_user) }

      it 'shows all medications' do
        get new_person_person_medication_path(admin_person)

        expect(response).to have_http_status(:ok)
        expect(response.body).to include(medications(:vitamin_d).name)
        expect(response.body).to include(foreign_medication.name)
      end
    end
  end

  describe 'POST /people/:person_id/person_medications' do
    context 'when user can create person medications and validation fails' do
      before { sign_in(adult_patient_user) }

      it 'shows only accessible medications again on re-render' do
        post person_person_medications_path(adult_patient_person),
             params: { person_medication: { medication_id: '', notes: '' } }

        expect(response).to have_http_status(:unprocessable_content)
        expect(response.body).to include(medications(:vitamin_d).name)
        expect(response.body).not_to include(foreign_medication.name)
      end

      it 're-renders the workflow modal for turbo stream requests' do
        post person_person_medications_path(adult_patient_person),
             params: { person_medication: { medication_id: '', notes: '' } },
             headers: {
               'Accept' => 'text/vnd.turbo-stream.html',
               'Turbo-Frame' => 'modal'
             }

        expect(response).to have_http_status(:unprocessable_content)
        expect(response.media_type).to eq('text/vnd.turbo-stream.html')
        expect(response.body).to include('Add Medication for')
        expect(response.body).to include('Choose a medication')
      end

      it 'requires an explicit dose selection before creating the medication' do
        post person_person_medications_path(adult_patient_person),
             params: {
               person_medication: {
                 medication_id: medications(:vitamin_d).id,
                 notes: 'No dose selected'
               }
             }

        expect(response).to have_http_status(:unprocessable_content)
        expect(response.body).to include('Dose amount can&#39;t be blank')
        expect(response.body).to include('Dose unit can&#39;t be blank')
      end

      it 'rejects a forged foreign medication_id' do
        expect do
          post person_person_medications_path(adult_patient_person),
               params: {
                 person_medication: {
                   medication_id: foreign_medication.id,
                   dose_amount: 1,
                   dose_unit: 'tablet'
                 }
               }
        end.not_to change(PersonMedication, :count)

        expect(response).to redirect_to(root_path)
      end
    end
  end
end
