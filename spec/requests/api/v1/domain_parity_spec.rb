# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 domain parity' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  let(:user) { users(:admin) }
  let(:login_data) { api_login(user) }
  let(:household_id) { login_data.dig('household', 'id') }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')) }

  it 'creates and reads dosage options using portable medication ids' do
    medication = medications(:paracetamol)

    post api_v1_household_dosage_options_path(household_id),
         params: {
           dosage_option: {
             medication_id: medication.portable_id,
             amount: 250,
             unit: 'mg',
             frequency: 'Twice daily',
             default_max_daily_doses: 2,
             default_min_hours_between_doses: 8,
             default_dose_cycle: 'daily'
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:created)
    portable_id = response.parsed_body.dig('data', 'portable_id')
    expect(response.parsed_body.dig('data', 'medication_portable_id')).to eq(medication.portable_id)

    get api_v1_household_dosage_option_path(household_id, portable_id), headers: headers, as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'amount')).to eq('250.0')
  end

  it 'creates health events with portable person and medication ids' do
    person = people(:john)
    medication = medications(:paracetamol)

    post api_v1_household_health_events_path(household_id),
         params: {
           health_event: {
             person_id: person.portable_id,
             event_kind: 'illness',
             severity: 'mild',
             title: 'Cold symptoms',
             notes: 'Managed at home',
             started_on: '2026-02-25',
             medication_ids: [medication.portable_id]
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:created)
    expect(response.parsed_body.dig('data', 'person_portable_id')).to eq(person.portable_id)
    expect(response.parsed_body.dig('data', 'medication_portable_ids')).to contain_exactly(medication.portable_id)
  end

  it 'records medication takes using portable schedule ids' do
    schedule = schedules(:john_paracetamol)

    expect do
      post api_v1_household_medication_takes_path(household_id),
           params: {
             medication_take: {
               client_uuid: SecureRandom.uuid,
               source_type: 'schedule',
               source_id: schedule.portable_id,
               taken_at: '2026-02-25T08:30:00Z'
             }
           },
           headers: headers,
           as: :json
    end.to change(MedicationTake, :count).by(1)

    expect(response).to have_http_status(:created)
    expect(response.parsed_body.dig('data', 'schedule_portable_id')).to eq(schedule.portable_id)
  end

  it 'pauses and resumes schedules by portable id' do
    schedule = schedules(:john_paracetamol)

    patch pause_api_v1_household_schedule_path(household_id, schedule.portable_id), headers: headers, as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'paused')).to be(true)

    patch resume_api_v1_household_schedule_path(household_id, schedule.portable_id), headers: headers, as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'paused')).to be(false)
  end

  it 'pauses, resumes, and reorders person medications by portable id' do
    first = create(:person_medication, :as_needed,
                   person: people(:john), medication: medications(:ibuprofen), position: 1)
    second = create(:person_medication, :as_needed,
                    person: people(:john), medication: medications(:paracetamol), position: 2)

    patch pause_api_v1_household_person_medication_path(household_id, second.portable_id), headers: headers, as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'paused')).to be(true)

    patch resume_api_v1_household_person_medication_path(household_id, second.portable_id), headers: headers, as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'paused')).to be(false)

    patch reorder_api_v1_household_person_medication_path(household_id, second.portable_id),
          params: { direction: 'up' },
          headers: headers,
          as: :json

    expect(response).to have_http_status(:ok)
    expect(first.reload.position).to eq(2)
    expect(second.reload.position).to eq(1)
  end

  it 'adjusts medication inventory by portable id' do
    medication = medications(:paracetamol)

    patch adjust_inventory_api_v1_household_medication_path(household_id, medication.portable_id),
          params: { adjustment: { new_quantity: 42, reason: 'Cycle count' } },
          headers: headers,
          as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'current_supply')).to eq('42.0')
  end

  it 'registers device tokens and push subscriptions through the API household scope' do
    post api_v1_household_native_device_tokens_path(household_id),
         params: { native_device_token: { device_token: 'api-device-token', platform: 'ios' } },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:created)
    expect(user.person.account.native_device_tokens.find_by(device_token: 'api-device-token')).to be_present

    post api_v1_household_push_subscription_path(household_id),
         params: {
           push_subscription: {
             endpoint: 'https://push.example.test/api-subscription',
             keys: { p256dh: 'p256dh', auth: 'auth' }
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:created)
    expect(
      user.person.account.push_subscriptions.find_by(endpoint: 'https://push.example.test/api-subscription')
    ).to be_present
  end

  it 'uses household-scoped medication lookup from the API' do
    responder = instance_double(
      MedicationFinderSearchResponder,
      call: MedicationFinderSearchResponder::Result.new(
        body: { results: [{ name: 'Calpol Six Plus' }], permissions: { can_create: true, can_update: false } },
        status: :ok
      )
    )

    allow(MedicationFinderSearchResponder).to receive(:new)
      .with(hash_including(medication_scope: kind_of(ActiveRecord::Relation)))
      .and_return(responder)

    get api_v1_household_medication_lookup_path(household_id),
        params: { q: 'calpol' },
        headers: headers,
        as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('results', 0, 'name')).to eq('Calpol Six Plus')
    expect(MedicationFinderSearchResponder).to have_received(:new)
  end

  it 'returns AI medication suggestions through the API when the feature is enabled' do
    suggestion = AiMedication::Suggestion.new(
      medication: { description: 'Paracetamol pain and fever relief' },
      doses: [],
      sources: []
    )
    service = instance_double(AiMedication::SuggestionService, call: suggestion)

    Household.find(household_id).update!(subscription_plan: 'family_plus')
    allow(ENV).to receive(:fetch).with('MEDTRACKER_AI_MEDICATION_HELP_ENABLED', 'false').and_return('true')
    allow(AiMedication::SuggestionService).to receive(:new).and_return(service)

    post api_v1_household_ai_medication_suggestions_path(household_id),
         params: { medication: { name: 'Calpol Six Plus' } },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'medication', 'description')).to eq('Paracetamol pain and fever relief')
  end

  it 'rejects cross-household portable ids as not found' do
    other_household = Household.create!(name: 'API Domain Other Household', slug: 'api-domain-other-household')
    other_medication = create(:medication, household: other_household)
    other_dosage = create(:dosage, medication: other_medication, household: other_household)

    get api_v1_household_dosage_option_path(household_id, other_dosage.portable_id), headers: headers, as: :json

    expect(response).to have_http_status(:not_found)
  end
end
