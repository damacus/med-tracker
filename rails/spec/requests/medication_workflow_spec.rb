# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication workflow' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :carer_relationships

  let(:admin_user) { users(:admin) }
  let(:adult_patient_user) { users(:adult_patient) }

  describe 'GET /add_medication' do
    it 'shows all addable people for an administrator' do
      sign_in(admin_user)

      get add_medication_path, params: { medication_id: medications(:paracetamol).id }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include(people(:john).name)
      expect(response.body).to include(people(:adult_patient_person).name)
    end

    it 'shows only policy-allowed people for a patient user' do
      sign_in(adult_patient_user)

      get add_medication_path

      expect(response).to have_http_status(:ok)
      expect(response.body).to include(people(:adult_patient_person).name)
      expect(response.body).not_to include(people(:john).name)
    end

    it 'keeps the current person-selection behavior when the experiment is off' do
      sign_in(admin_user)

      get add_medication_path, params: { person_id: people(:john).id }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include(people(:john).name)
    end

    it 'carries authorized context into the canonical assignment workflow' do
      admin_user.person.account.update!(medication_launcher_variant: 'context_aware')
      person = people(:john)
      medication = medications(:paracetamol)
      sign_in(admin_user)

      get add_medication_path,
          params: {
            person_id: person.id,
            medication_id: medication.id,
            intent: 'assign_medication',
            return_to: medications_path
          }

      expect(response).to redirect_to(
        new_person_medication_assignment_path(
          person,
          source: :workflow,
          medication_id: medication.id,
          return_to: medications_path
        )
      )
    end

    it 'discards an external return destination' do
      admin_user.person.account.update!(medication_launcher_variant: 'context_aware')
      person = people(:john)
      sign_in(admin_user)

      get add_medication_path,
          params: { person_id: person.id, return_to: 'https://example.com/medications' }

      expect(response).to redirect_to(
        new_person_medication_assignment_path(person, source: :workflow)
      )
    end

    it 'falls back to person selection for unsupported intent' do
      admin_user.person.account.update!(medication_launcher_variant: 'context_aware')
      sign_in(admin_user)

      get add_medication_path, params: { person_id: people(:john).id, intent: 'invent_something' }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include(people(:john).name)
    end

    it 'falls back without disclosing a stale person' do
      admin_user.person.account.update!(medication_launcher_variant: 'context_aware')
      sign_in(admin_user)

      get add_medication_path, params: { person_id: 'missing-person' }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include(people(:john).name)
      expect(response.body).not_to include('missing-person')
    end

    it 'falls back without disclosing an unauthorized person' do
      adult_patient_user.person.account.update!(medication_launcher_variant: 'context_aware')
      sign_in(adult_patient_user)

      get add_medication_path, params: { person_id: people(:john).id }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include(people(:adult_patient_person).name)
      expect(response.body).not_to include(people(:john).name)
    end
  end
end
