# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medications reorder status with turbo_stream' do
  fixtures :accounts, :people, :users, :locations, :medications

  let(:admin) { users(:admin) }
  let(:medication) { medications(:paracetamol) }

  before { sign_in(admin) }

  describe 'PATCH /medications/:id/mark_as_ordered' do
    it 'returns turbo_stream and updates show container and flash' do
      patch mark_as_ordered_medication_path(medication), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"medication_show_#{medication.id}\"")
      expect(response.body).to include('target="flash"')
      expect(medication.reload.reorder_status).to eq('ordered')
    end
  end

  describe 'PATCH /medications/:id/mark_as_received' do
    it 'returns turbo_stream and updates show container and flash' do
      medication.update!(reorder_status: :ordered, ordered_at: Time.current)

      patch mark_as_received_medication_path(medication), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"medication_show_#{medication.id}\"")
      expect(response.body).to include('target="flash"')
      expect(medication.reload.reorder_status).to eq('received')
    end
  end
end
