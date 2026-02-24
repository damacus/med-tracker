# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medicines refill' do
  fixtures :accounts, :people, :users, :locations, :medicines

  let(:admin) { users(:admin) }
  let(:medicine) { medicines(:paracetamol) }

  before do
    sign_in(admin)
  end

  describe 'PATCH /medicines/:id/refill' do
    it 'updates both current_supply and stock and records a restock audit event' do
      expect do
        patch refill_medicine_path(medicine), params: { refill: { quantity: 10, restock_date: Date.current.to_s } }
      end.to change(PaperTrail::Version.where(item_type: 'Medicine'), :count).by(1)

      expect(response).to redirect_to(medicine_path(medicine))

      medicine.reload
      expect(medicine.current_supply).to eq(90)
      expect(medicine.stock).to eq(110)

      version = PaperTrail::Version.where(item_type: 'Medicine').last
      expect(version.event).to include('restock')
      expect(version.event).to include('qty: 10')
      expect(version.event).to include(Date.current.iso8601)
      expect(version.whodunnit).to eq(admin.id.to_s)
    end

    it 'returns unprocessable content and does not update stock when quantity is invalid' do
      expect do
        patch refill_medicine_path(medicine), params: { refill: { quantity: 0, restock_date: Date.current.to_s } }
      end.not_to(change { medicine.reload.current_supply })

      expect(response).to have_http_status(:unprocessable_content)
    end
  end
end
