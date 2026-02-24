# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'PersonMedicinesController medicine options' do
  fixtures :accounts, :people, :locations, :medicines, :users, :carer_relationships

  let(:adult_patient_user) { users(:adult_patient) }
  let(:adult_patient_person) { people(:adult_patient_person) }
  let(:admin_user) { users(:admin) }
  let(:admin_person) { people(:admin) }

  describe 'GET /people/:person_id/person_medicines/new' do
    context 'when user can create person medicines for themselves' do
      before { sign_in(adult_patient_user) }

      it 'shows all medicines as options' do
        get new_person_person_medicine_path(adult_patient_person)

        expect(response).to have_http_status(:ok)
        Medicine.find_each do |medicine|
          expect(response.body).to include(medicine.name)
        end
      end
    end

    context 'when user is an administrator' do
      before { sign_in(admin_user) }

      it 'shows all medicines' do
        get new_person_person_medicine_path(admin_person)

        expect(response).to have_http_status(:ok)
        Medicine.find_each do |medicine|
          expect(response.body).to include(medicine.name)
        end
      end
    end
  end

  describe 'POST /people/:person_id/person_medicines' do
    context 'when user can create person medicines and validation fails' do
      before { sign_in(adult_patient_user) }

      it 'shows all medicines again on re-render' do
        post person_person_medicines_path(adult_patient_person),
             params: { person_medicine: { medicine_id: '', notes: '' } }

        expect(response).to have_http_status(:unprocessable_content)
        Medicine.find_each do |medicine|
          expect(response.body).to include(medicine.name)
        end
      end
    end
  end
end
