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
    expect(response.media_type).to eq('application/fhir+json')
    expect(fhir_json).to include('resourceType' => 'OperationOutcome')
  end

  it 'returns FHIR metadata' do
    get '/api/fhir/R4/metadata', headers: headers, as: :json

    expect(response).to have_http_status(:ok)
    expect(response.media_type).to eq('application/fhir+json')
    expect(fhir_json).to include(
      'resourceType' => 'CapabilityStatement',
      'fhirVersion' => '4.0.1',
      'format' => include('json', 'application/fhir+json')
    )
    patient = fhir_json.dig('rest', 0, 'resource').find { |resource| resource.fetch('type') == 'Patient' }
    expect(patient.fetch('searchParam').pluck('name')).to include('_id', 'name', 'birthdate')
    expect(fhir_json.dig('rest', 0, 'security', 'service', 0, 'coding', 0)).to include(
      'code' => 'SMART-on-FHIR'
    )
  end

  it 'searches and reads patients with household scoping' do
    person = people(:john)

    get "/api/fhir/R4/Patient?name=#{person.name}&_count=1", headers: headers

    expect(response).to have_http_status(:ok)
    expect(response.media_type).to eq('application/fhir+json')
    expect(fhir_json).to include('resourceType' => 'Bundle', 'type' => 'searchset')
    expect(fhir_json.fetch('link').pluck('relation')).to include('self')
    expect(fhir_json.fetch('entry')).to all(include('search' => { 'mode' => 'match' }))
    expect(fhir_json.fetch('entry').map { |entry| entry.dig('resource', 'id') }).to contain_exactly(
      person.portable_id
    )

    get '/api/fhir/R4/Patient?_count=1', headers: headers

    expect(fhir_json.fetch('link').pluck('relation')).to include('next')

    get "/api/fhir/R4/Patient/#{person.portable_id}", headers: headers, as: :json

    expect(response).to have_http_status(:ok)
    expect(fhir_json).to include('resourceType' => 'Patient', 'id' => person.portable_id)
  end

  it 'returns medication, request, statement, and administration resources with references' do
    schedule = schedules(:john_paracetamol)
    person_medication = create(:person_medication, person: people(:john), medication: medications(:ibuprofen))
    take = create(:medication_take, schedule: schedule, household: schedule.household, taken_at: Time.current)

    get "/api/fhir/R4/Medication/#{schedule.medication.portable_id}", headers: headers, as: :json
    expect(fhir_json).to include('resourceType' => 'Medication', 'id' => schedule.medication.portable_id)

    get "/api/fhir/R4/MedicationRequest/#{schedule.portable_id}", headers: headers, as: :json
    expect(fhir_json.dig('subject', 'reference')).to eq("Patient/#{schedule.person.portable_id}")
    expect(fhir_json.dig('medicationReference', 'reference')).to eq(
      "Medication/#{schedule.medication.portable_id}"
    )

    get "/api/fhir/R4/MedicationStatement/#{person_medication.portable_id}", headers: headers, as: :json
    expect(fhir_json.dig('subject', 'reference')).to eq("Patient/#{person_medication.person.portable_id}")

    get "/api/fhir/R4/MedicationAdministration/#{take.portable_id}", headers: headers, as: :json
    expect(fhir_json.dig('subject', 'reference')).to eq("Patient/#{schedule.person.portable_id}")
  end

  it 'filters resources by supported FHIR search parameters' do
    medication = medications(:paracetamol)
    medication.update!(dmd_code: '123456', dmd_system: 'https://dmd.nhs.uk', dmd_concept_class: 'VMP')
    schedule = schedules(:john_paracetamol)
    person_medication = create(:person_medication, person: people(:john), medication: medications(:ibuprofen))
    stopped_schedule = create(
      :schedule,
      active: false,
      person: schedule.person,
      medication: schedule.medication,
      household: schedule.household
    )
    take = create(
      :medication_take,
      schedule: schedule,
      household: schedule.household,
      taken_at: Time.zone.local(2026, 1, 2, 9)
    )

    get '/api/fhir/R4/Medication?code=123456', headers: headers
    expect(fhir_json.fetch('entry').map { |entry| entry.dig('resource', 'id') }).to include(medication.portable_id)

    query = {
      patient: "Patient/#{schedule.person.portable_id}",
      medication: "Medication/#{medication.portable_id}"
    }.to_query
    get "/api/fhir/R4/MedicationRequest?#{query}", headers: headers
    expect(fhir_json.fetch('entry').map { |entry| entry.dig('resource', 'id') }).to include(schedule.portable_id)

    get '/api/fhir/R4/MedicationRequest?status=active', headers: headers
    expect(fhir_json.fetch('entry').map { |entry| entry.dig('resource', 'id') }).to include(schedule.portable_id)

    get '/api/fhir/R4/MedicationRequest?status=stopped', headers: headers
    expect(fhir_json.fetch('entry').map { |entry| entry.dig('resource', 'id') }).to include(
      stopped_schedule.portable_id
    )

    get '/api/fhir/R4/MedicationRequest?status=draft', headers: headers
    expect(response).to have_http_status(:unprocessable_content)

    get '/api/fhir/R4/MedicationStatement?status=active', headers: headers
    expect(fhir_json.fetch('entry').map { |entry| entry.dig('resource', 'id') }).to include(
      person_medication.portable_id
    )

    person_medication.update!(active: false)

    get '/api/fhir/R4/MedicationStatement?status=stopped', headers: headers
    expect(fhir_json.fetch('entry').map { |entry| entry.dig('resource', 'id') }).to include(
      person_medication.portable_id
    )

    get '/api/fhir/R4/MedicationStatement?status=entered-in-error', headers: headers
    expect(response).to have_http_status(:unprocessable_content)

    get "/api/fhir/R4/MedicationAdministration?patient=#{schedule.person.portable_id}&date=2026-01-02&status=completed",
        headers: headers
    expect(fhir_json.fetch('entry').map { |entry| entry.dig('resource', 'id') }).to include(take.portable_id)

    get '/api/fhir/R4/MedicationAdministration?status=in-progress', headers: headers
    expect(response).to have_http_status(:unprocessable_content)
  end

  it 'returns OperationOutcome for unsupported FHIR formats and search parameters' do
    get '/api/fhir/R4/Patient?family=Smith', headers: headers

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.media_type).to eq('application/fhir+json')
    expect(fhir_json).to include('resourceType' => 'OperationOutcome')

    get '/api/fhir/R4/Patient?_format=xml', headers: headers

    expect(response).to have_http_status(:not_acceptable)
    expect(fhir_json.dig('issue', 0, 'code')).to eq('not-supported')
  end

  it 'returns not found for inaccessible resources' do
    other_household = Household.create!(name: 'FHIR Other Household', slug: 'fhir-other-household')
    other_person = create(:person, household: other_household)

    get "/api/fhir/R4/Patient/#{other_person.portable_id}", headers: headers, as: :json

    expect(response).to have_http_status(:not_found)
    expect(fhir_json).to include('resourceType' => 'OperationOutcome')
  end

  def fhir_json
    ActiveSupport::JSON.decode(response.body)
  end
end
