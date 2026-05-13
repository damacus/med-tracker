# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medications refill' do
  fixtures :accounts, :people, :users, :locations, :medications

  let(:admin) { users(:admin) }
  let(:parent) { users(:parent) }
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

    it 'allows a parent to restock an accessible medication' do
      sign_in(parent)

      patch refill_medication_path(medication), params: { refill: { quantity: 10, restock_date: Date.current.to_s } }

      expect(response).to redirect_to(medication_path(medication))
      expect(medication.reload.current_supply).to eq(90)
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
      expect(response.body).to include("target=\"medication_#{medication.id}\"")
      expect(response.body).to include('target="flash"')
    end

    it 'returns turbo_stream and unprocessable content on invalid quantity' do
      patch refill_medication_path(medication),
            params: { refill: { quantity: 0, restock_date: Date.current.to_s } },
            headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"medication_show_#{medication.id}\"")
      expect(response.body).to include("target=\"medication_#{medication.id}\"")
      expect(response.body).to include('target="flash"')
    end
  end

  describe 'POST /medications/scan_restock' do
    it 'adds scanned stock to the matching medication' do
      medication.update!(barcode: '5012345678901', current_supply: 0, supply_at_last_restock: 30)

      expect do
        post scan_restock_medications_path,
             params: { inventory_scan: { barcode: '5012345678901', quantity: 30 } }
      end.to change(PaperTrail::Version.where(item_type: 'Medication'), :count).by(1)

      expect(response).to redirect_to(medication_path(medication))
      expect(medication.reload).to have_attributes(
        current_supply: 30,
        supply_at_last_restock: 30
      )
    end

    it 'allows a parent to add scanned stock to an accessible medication' do
      sign_in(parent)
      medication.update!(barcode: '5012345678901', current_supply: 0, supply_at_last_restock: 30)

      post scan_restock_medications_path,
           params: { inventory_scan: { barcode: '5012345678901', quantity: 30 } }

      expect(response).to redirect_to(medication_path(medication))
      expect(medication.reload).to have_attributes(
        current_supply: 30,
        supply_at_last_restock: 30
      )
    end

    it 'adds scanned stock to an existing medication matched from barcode metadata' do
      movicol = medications(:movicol)
      movicol.update!(current_supply: 0, supply_at_last_restock: 30)
      NhsDmdBarcode.create!(
        gtin: '5012345678901',
        code: '3366911000001108',
        display: movicol.name,
        system: 'https://dmd.nhs.uk',
        concept_class: 'AMPP'
      )

      post scan_restock_medications_path,
           params: { inventory_scan: { barcode: '5012345678901', quantity: 30 } }

      expect(response).to redirect_to(medication_path(movicol))
      expect(movicol.reload).to have_attributes(
        current_supply: 30,
        supply_at_last_restock: 30
      )
    end

    it 'does not restock when the scanned barcode is unknown' do
      expect do
        post scan_restock_medications_path,
             params: { inventory_scan: { barcode: '5012345678901', quantity: 30 } }
      end.not_to(change { medication.reload.current_supply })

      expect(response).to redirect_to(medications_path)
      expect(flash[:alert]).to eq('No medication matched that barcode.')
    end
  end

  describe 'GET /medications/scan_restock_match' do
    it 'serves the .json path used by the scan-stock modal' do
      medication.update!(barcode: '5012345678901')

      get '/medications/scan_restock_match.json', params: { q: '5012345678901' }

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body).to include('matched' => true)
    end

    it 'returns the matching medication for a direct barcode match' do
      medication.update!(barcode: '5012345678901')

      get scan_restock_match_medications_path(format: :json), params: { q: '5012345678901' }

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body).to include('matched' => true)
      expect(response.parsed_body.dig('medication', 'id')).to eq(medication.id)
      expect(response.parsed_body.dig('medication', 'name')).to eq(medication.display_name)
      expect(response.parsed_body.dig('medication', 'location')).to eq('Home')
    end

    it 'returns the matching medication for a parent who can restock but cannot update details' do
      sign_in(parent)
      medication.update!(barcode: '5012345678901')

      get scan_restock_match_medications_path(format: :json), params: { q: '5012345678901' }

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body).to include('matched' => true)
      expect(response.parsed_body.dig('medication', 'id')).to eq(medication.id)
    end

    it 'returns the matching medication from barcode metadata' do
      movicol = medications(:movicol)
      NhsDmdBarcode.create!(
        gtin: '5012345678901',
        code: '3366911000001108',
        display: movicol.name,
        system: 'https://dmd.nhs.uk',
        concept_class: 'AMPP'
      )

      get scan_restock_match_medications_path(format: :json), params: { q: '5012345678901' }

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body).to include('matched' => true)
      expect(response.parsed_body.dig('medication', 'id')).to eq(movicol.id)
    end

    it 'returns no match for an unknown barcode' do
      get scan_restock_match_medications_path(format: :json), params: { q: '5012345678901' }

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body).to eq('matched' => false)
    end

    it 'does not expose inaccessible medication matches' do
      foreign_location = Location.create!(name: 'Foreign')
      create(:medication, barcode: '5012345678901', location: foreign_location)
      sign_in(users(:carer))

      get scan_restock_match_medications_path(format: :json), params: { q: '5012345678901' }

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body).to eq('matched' => false)
    end
  end
end
