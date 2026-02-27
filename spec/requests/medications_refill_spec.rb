# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medications refill' do
  fixtures :accounts, :people, :users, :locations, :medications

  let(:admin) { users(:admin) }
  let(:medication) { medications(:paracetamol) }

  before do
    sign_in(admin)
  end

  describe 'PATCH /medications/:id/refill' do
    it 'updates current_supply and records a restock audit event' do
      expect do
        patch refill_medication_path(medication), params: { refill: { quantity: 10, restock_date: Date.current.to_s } }
      end.to change(PaperTrail::Version.where(item_type: 'Medication'), :count).by(1)

      expect(response).to redirect_to(medication_path(medication))

      medication.reload
      expect(medication.current_supply).to eq(90)
      expect(medication.supply_at_last_restock).to eq(90)

      version = PaperTrail::Version.where(item_type: 'Medication').last
      expect(version.event).to include('restock')
      expect(version.event).to include('qty: 10')
      expect(version.event).to include(Date.current.iso8601)
      expect(version.whodunnit).to eq(admin.id.to_s)
    end

    it 'returns unprocessable content and does not update supply when quantity is invalid' do
      expect do
        patch refill_medication_path(medication), params: { refill: { quantity: 0, restock_date: Date.current.to_s } }
      end.not_to(change { medication.reload.current_supply })

      expect(response).to have_http_status(:unprocessable_content)
    end

    it 'returns turbo_stream and updates show container and flash on success' do
      patch refill_medication_path(medication),
            params: { refill: { quantity: 10, restock_date: Date.current.to_s } },
            headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"medication_show_#{medication.id}\"")
      expect(response.body).to include('target="flash"')
    end

    it 'returns turbo_stream and unprocessable content on invalid quantity' do
      patch refill_medication_path(medication),
            params: { refill: { quantity: 0, restock_date: Date.current.to_s } },
            headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"medication_show_#{medication.id}\"")
      expect(response.body).to include('target="flash"')
    end
  end
end
