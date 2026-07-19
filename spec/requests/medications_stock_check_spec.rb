# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medications stock check' do
  fixtures :accounts, :people, :users, :locations, :medications

  let(:admin) { users(:admin) }
  let(:parent) { users(:parent) }
  let(:paracetamol) { household_medication(medications(:paracetamol)) }
  let(:aspirin) { household_medication(medications(:aspirin)) }

  before do
    sign_in(admin)
  end

  describe 'GET /medications/stock_check' do
    it 'renders the household inventory administration workspace' do
      get stock_check_medications_path(location_id: household_location(locations(:home)).id)

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('Inventory administration')
      expect(response.body).to include('Stock amendment mode')
      expect(response.body).to include(paracetamol.display_name)
      expect(response.body).to include('data-controller="stock-check"')
      expect(response.body).to include('data-stock-check-units-value="units"')
      expect(response.body).to include('>0 units</strong>')
      expect(response.body).not_to include('>-0 units</strong>')
    end

    it 'links to stock check from the medicines inventory' do
      get medications_path

      expect(response.body).to include(stock_check_medications_path)
      expect(response.body).to include('Stock check')
    end

    it 'does not allow a household member to open the workspace' do
      sign_in(parent)

      get stock_check_medications_path

      expect(response).to redirect_to(root_path)
    end
  end

  describe 'PATCH /medications/bulk_adjust_inventory' do
    it 'applies every selected amendment and records the shared reason' do
      expect do
        patch bulk_adjust_inventory_medications_path, params: {
          stock_check: {
            reason: 'House stock check',
            adjustments: {
              paracetamol.id.to_s => '74',
              aspirin.id.to_s => '0'
            }
          }
        }
      end.to change(PaperTrail::Version.where(item_type: 'Medication'), :count).by(2)

      expect(response).to redirect_to(stock_check_medications_path)
      expect(paracetamol.reload.current_supply).to eq(74)
      expect(aspirin.reload.current_supply).to eq(0)
      expect(PaperTrail::Version.where(item: paracetamol).last.event).to include('reason: House stock check')
    end

    it 'renders an error and rolls back every amendment when one quantity is invalid' do
      patch bulk_adjust_inventory_medications_path, params: {
        stock_check: {
          reason: 'House stock check',
          adjustments: {
            paracetamol.id.to_s => '74',
            aspirin.id.to_s => '-1'
          }
        }
      }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('Quantity cannot be negative')
      expect(paracetamol.reload.current_supply).to eq(80)
      expect(aspirin.reload.current_supply).to eq(25)
    end
  end
end
