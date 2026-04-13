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
  end
end
