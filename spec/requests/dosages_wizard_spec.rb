# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication wizard dose option follow-up' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications

  before { sign_in(users(:admin)) }

  it 'replaces the wizard content with a medication-owned dose options step' do
    post medications_path,
         params: {
           wizard: 'true',
           medication: {
             name: 'Wizard Medication',
             category: 'Vitamin',
             dosage_amount: '2.5',
             dosage_unit: 'ml',
             current_supply: '10',
             reorder_threshold: '1',
             location_id: locations(:home).id
           }
         },
         headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

    expect(response).to have_http_status(:ok)
    expect(response.content_type).to include('text/vnd.turbo-stream.html')

    medication = Medication.order(:id).last
    body = response.body
    expect(body).to include('turbo-stream')
    expect(body).to include('target="wizard-content"')
    expect(body).to include('Manage dose options')
    expect(body).to include(edit_medication_path(medication, return_to: medication_path(medication)))
    expect(body).not_to include('target="dosage-form"')
    expect(body).not_to include('target="dosage-list"')
  end

  it 'falls back to the medication page for non-turbo requests' do
    post medications_path,
         params: {
           wizard: 'true',
           medication: {
             name: 'Redirected Wizard Medication',
             category: 'Vitamin',
             dosage_amount: '5',
             dosage_unit: 'ml',
             current_supply: '10',
             reorder_threshold: '1',
             location_id: locations(:home).id
           }
         }

    expect(response).to redirect_to(medication_path(Medication.order(:id).last))
  end
end
