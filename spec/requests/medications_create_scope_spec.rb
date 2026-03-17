# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication creation scope' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :carer_relationships

  let(:parent_user) { users(:jane) }
  let!(:foreign_location) { locations(:grandmas) }

  before { sign_in(parent_user) }

  describe 'GET /medications/new' do
    it 'shows only authorized locations in the form' do
      get new_medication_path

      expect(response).to have_http_status(:ok)
      expect(response.body).to include(locations(:home).name)
      expect(response.body).to include(locations(:school).name)
      expect(response.body).not_to include(foreign_location.name)
    end
  end

  describe 'POST /medications' do
    it 'creates a medication in an authorized location' do
      expect do
        post medications_path, params: {
          medication: {
            name: 'Scoped Parent Medication',
            category: 'Vitamin',
            dosage_amount: 5,
            dosage_unit: 'ml',
            current_supply: 10,
            reorder_threshold: 1,
            location_id: locations(:home).id
          }
        }
      end.to change(Medication, :count).by(1)

      expect(response).to redirect_to(medication_path(Medication.last))
      expect(Medication.last.location).to eq(locations(:home))
    end

    it 'rejects a forged foreign location_id' do
      expect do
        post medications_path, params: {
          medication: {
            name: 'Foreign Location Medication',
            category: 'Vitamin',
            dosage_amount: 5,
            dosage_unit: 'ml',
            current_supply: 10,
            reorder_threshold: 1,
            location_id: foreign_location.id
          }
        }
      end.not_to change(Medication, :count)

      expect(response).to redirect_to(root_path)
    end
  end
end
