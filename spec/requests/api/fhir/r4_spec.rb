# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'FHIR R4 API' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  let(:user) { users(:admin) }
  let(:login_data) { api_login(user) }
  let(:household_id) { login_data.dig('household', 'id') }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')) }

  it 'requires bearer authentication' do
    get '/api/fhir/R4/metadata', as: :json

    expect(response).to have_http_status(:unauthorized)
  end

  it 'returns FHIR metadata' do
    get '/api/fhir/R4/metadata', headers: headers, as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body).to include('resourceType' => 'CapabilityStatement', 'fhirVersion' => '4.0.1')
  end

  it 'searches and reads patients with household scoping' do
    person = people(:john)

    get '/api/fhir/R4/Patient', headers: headers, as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('resourceType')).to eq('Bundle')
    expect(response.parsed_body.fetch('entry').map { |entry| entry.dig('resource', 'id') }).to include(
      person.portable_id
    )

    get "/api/fhir/R4/Patient/#{person.portable_id}", headers: headers, as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body).to include('resourceType' => 'Patient', 'id' => person.portable_id)
  end

  it 'returns medication, request, statement, and administration resources with references' do
    schedule = schedules(:john_paracetamol)
    person_medication = create(:person_medication, person: people(:john), medication: medications(:ibuprofen))
    take = create(:medication_take, schedule: schedule, household: schedule.household, taken_at: Time.current)

    get "/api/fhir/R4/Medication/#{schedule.medication.portable_id}", headers: headers, as: :json
    expect(response.parsed_body).to include('resourceType' => 'Medication', 'id' => schedule.medication.portable_id)

    get "/api/fhir/R4/MedicationRequest/#{schedule.portable_id}", headers: headers, as: :json
    expect(response.parsed_body.dig('subject', 'reference')).to eq("Patient/#{schedule.person.portable_id}")
    expect(response.parsed_body.dig('medicationReference', 'reference')).to eq(
      "Medication/#{schedule.medication.portable_id}"
    )

    get "/api/fhir/R4/MedicationStatement/#{person_medication.portable_id}", headers: headers, as: :json
    expect(response.parsed_body.dig('subject', 'reference')).to eq("Patient/#{person_medication.person.portable_id}")

    get "/api/fhir/R4/MedicationAdministration/#{take.portable_id}", headers: headers, as: :json
    expect(response.parsed_body.dig('subject', 'reference')).to eq("Patient/#{schedule.person.portable_id}")
  end

  it 'returns not found for inaccessible resources' do
    other_household = Household.create!(name: 'FHIR Other Household', slug: 'fhir-other-household')
    other_person = create(:person, household: other_household)

    get "/api/fhir/R4/Patient/#{other_person.portable_id}", headers: headers, as: :json

    expect(response).to have_http_status(:not_found)
  end
end
