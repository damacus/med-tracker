# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'PersonMedicinesController policy scoping' do
  fixtures :accounts, :people, :medicines, :users, :carer_relationships

  let(:adult_patient_user) { users(:adult_patient) }
  let(:adult_patient_person) { people(:adult_patient_person) }
  let(:admin_user) { users(:admin) }
  let(:admin_person) { people(:admin) }

  describe 'GET /people/:person_id/person_medicines/new' do
    context 'when user role is excluded from MedicinePolicy::Scope' do
      before { sign_in(adult_patient_user) }

      it 'uses policy_scope to restrict visible medicines' do
        get new_person_person_medicine_path(adult_patient_person)

        expect(response).to have_http_status(:ok)
        Medicine.find_each do |medicine|
          expect(response.body).not_to include(medicine.name)
        end
      end
    end

    context 'when user role is included in MedicinePolicy::Scope' do
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
    context 'when user role is excluded from MedicinePolicy::Scope and validation fails' do
      before { sign_in(adult_patient_user) }

      it 'uses policy_scope for medicines on re-render' do
        post person_person_medicines_path(adult_patient_person),
             params: { person_medicine: { medicine_id: '', notes: '' } }

        expect(response).to have_http_status(:unprocessable_content)
        Medicine.find_each do |medicine|
          expect(response.body).not_to include(medicine.name)
        end
      end
    end
  end
end
