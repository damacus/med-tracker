require 'rails_helper'

RSpec.describe 'API v1 dose outcome writes' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  let(:login_data) { api_login(users(:admin)) }
  let(:household_id) { login_data.dig('household', 'id') }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')).merge('Idempotency-Key' => SecureRandom.uuid) }
  let(:source) { schedules(:john_movicol) }
  let(:path) { "/api/v1/households/#{household_id}/schedules/#{source.id}/dose_occurrences/not_taken" }

  def occurrence_key(date = Date.current)
    MedicationAdministration::OccurrenceProjection.new(source: source.reload, start_date: date,
                                                       end_date: date).call.first.key
  end

  def submit(key: occurrence_key, **attributes)
    post path, params: { dose_occurrence: { key: key, reason: 'unwell', note: 'Resting' }.merge(attributes) },
               headers: headers, as: :json
  end

  it 'records one attributable outcome without administering or reducing stock' do
    headers
    stock = source.medication.current_supply
    expect { submit }.not_to change(MedicationTake, :count)
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('data')).to include('outcome' => 'not_taken', 'reason' => 'unwell',
                                                          'note' => 'Resting')
    record = source.medication_dose_occurrences.sole
    expect(record.resolved_by_membership.account).to eq(users(:admin).person.account)
    expect(source.medication.reload.current_supply).to eq(stock)
    expect(response.headers['ETag']).to eq(Api::RecordEtag.for(record))
  end

  it 'replays an identical request without another outcome or version' do
    submit
    expect(response).to have_http_status(:ok)
    expect { submit }.not_to change(PaperTrail::Version, :count)
    expect(response).to have_http_status(:ok)
    expect(response.headers['Idempotency-Replayed']).to eq('true')
    expect(source.medication_dose_occurrences.count).to eq(1)
  end

  it 'accepts a valid overdue occurrence' do
    source.update!(start_date: Date.yesterday)
    submit(key: occurrence_key(Date.yesterday))
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'window_starts_on')).to eq(Date.yesterday.iso8601)
  end

  it 'rejects a competing resolution without changing the saved context' do
    submit
    headers['Idempotency-Key'] = SecureRandom.uuid
    submit(reason: 'refused')
    expect(response).to have_http_status(:conflict)
    expect(response.parsed_body.dig('error', 'code')).to eq('already_resolved')
    expect(source.medication_dose_occurrences.sole.reason).to eq('unwell')
  end

  it 'checks person access again before replaying a saved response' do
    submit
    membership = users(:admin).person.account.first_active_household_membership
    PersonAccessGrant.where(household_membership: membership, person_id: source.person_id).find_each do |grant|
      grant.update!(revoked_at: Time.current)
    end
    submit
    expect(response).to have_http_status(:not_found)
    expect(response.body).not_to include('Resting')
  end

  it 'rejects future occurrences without persisting an outcome' do
    expect { submit(key: occurrence_key(Date.tomorrow)) }.not_to change(MedicationDoseOccurrence, :count)
    expect(response).to have_http_status(:unprocessable_content)
    expect(response.parsed_body.dig('error', 'code')).to eq('invalid_occurrence')
  end

  it 'rejects unsupported context without echoing it in an error' do
    expect { submit(reason: 'private clinical text') }.not_to change(MedicationDoseOccurrence, :count)
    expect(response).to have_http_status(:unprocessable_content)
    expect(response.body).not_to include('private clinical text')
  end

  it 'hides malformed or mismatched occurrence identities' do
    submit(key: 'private clinical text')
    expect(response).to have_http_status(:unprocessable_content)
    expect(response.body).not_to include('private clinical text')
  end

  it 'denies resolution when the current grant permits viewing only' do
    headers
    membership = users(:admin).person.account.first_active_household_membership
    PersonAccessGrant.where(household_membership: membership, person_id: source.person_id).find_each do |grant|
      grant.update!(access_level: 'view')
    end
    expect { submit }.not_to change(MedicationDoseOccurrence, :count)
    expect(response).to have_http_status(:forbidden)
  end
end
