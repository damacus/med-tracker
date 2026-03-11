# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Wizard dosage creation' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications

  before { sign_in(users(:admin)) }

  let(:medication) { medications(:paracetamol) }

  it 'returns turbo_stream that appends the new dosage row and resets the form' do
    post medication_dosages_path(medication),
         params: {
           wizard: 'true',
           dosage: {
             amount: '2.5',
             unit: 'ml',
             frequency: 'Once daily',
             default_for_adults: '1'
           }
         },
         headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

    expect(response).to have_http_status(:ok)
    expect(response.content_type).to include('text/vnd.turbo-stream.html')

    body = response.body
    expect(body).to include('turbo-stream')
    expect(body).to include('action="append"')
    expect(body).to include('target="dosage-list"')
    expect(body).to include('2.5')
    expect(body).to include('ml')
    expect(body).to include('action="replace"')
    expect(body).to include('target="dosage-form"')
  end

  it 'redirects to medication page without turbo stream header' do
    post medication_dosages_path(medication),
         params: {
           wizard: 'true',
           dosage: {
             amount: '5',
             unit: 'ml',
             frequency: 'Twice daily'
           }
         }

    expect(response).to redirect_to(medication_path(medication))
  end
end
