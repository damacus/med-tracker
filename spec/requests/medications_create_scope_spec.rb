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

    it 'prefills the medication name and barcode from the finder selection' do
      get new_medication_path, params: {
        name: 'Laxido Orange oral powder sachets (Galen Ltd)',
        barcode: '5016298210989',
        dmd_code: '13629411000001105',
        dmd_system: 'https://dmd.nhs.uk',
        dmd_concept_class: 'AMPP'
      }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('data-testid="medication-wizard-form"')
      expect(response.body).to include('value="Laxido Orange oral powder sachets (Galen Ltd)"')
      expect(response.body).to include('name="medication[barcode]"')
      expect(response.body).to include('value="5016298210989"')
      expect(response.body).to include('name="medication[dmd_code]"')
      expect(response.body).to include('value="13629411000001105"')
      expect(response.body).to include('name="medication[dmd_system]"')
      expect(response.body).to include('name="medication[dmd_concept_class]"')
    end

    it 'ignores an invalid non-GTIN barcode from the finder selection' do
      get new_medication_path, params: {
        name: 'Aspirin 300mg tablets',
        barcode: '1234567890'
      }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('data-testid="medication-wizard-form"')
      expect(response.body).to include('value="Aspirin 300mg tablets"')
      expect(response.body).not_to include('name="medication[barcode]"')
      expect(response.body).not_to include('value="1234567890"')
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

    it 'persists a barcode selected from the finder flow' do
      expect do
        post medications_path, params: {
          medication: {
            name: 'Laxido Orange oral powder sachets (Galen Ltd)',
            barcode: '5016298210989',
            dmd_code: '13629411000001105',
            dmd_system: 'https://dmd.nhs.uk',
            dmd_concept_class: 'AMPP',
            category: 'Osmotic Laxative',
            dosage_amount: 1,
            dosage_unit: 'sachet',
            current_supply: 30,
            reorder_threshold: 5,
            location_id: locations(:home).id
          }
        }
      end.to change(Medication, :count).by(1)

      expect(Medication.last.barcode).to eq('5016298210989')
      expect(Medication.last.dmd_code).to eq('13629411000001105')
      expect(Medication.last.dmd_system).to eq('https://dmd.nhs.uk')
      expect(Medication.last.dmd_concept_class).to eq('AMPP')
    end

    it 'shows a friendly error when the barcode is already used in another inaccessible inventory item' do
      create(:medication, barcode: '5016298210989', location: foreign_location)

      expect do
        post medications_path, params: {
          medication: {
            name: 'Laxido Orange oral powder sachets (Galen Ltd)',
            barcode: '5016298210989',
            category: 'Osmotic Laxative',
            dosage_amount: 1,
            dosage_unit: 'sachet',
            current_supply: 30,
            reorder_threshold: 5,
            location_id: locations(:home).id
          }
        }
      end.not_to change(Medication, :count)

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('Barcode is already linked to another medication in inventory')
      expect(response.body).not_to include('has already been taken')
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
