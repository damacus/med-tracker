require 'rails_helper'

RSpec.describe 'API v1 dose outcome corrections' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  let(:login_data) { api_login(users(:admin)) }
  let(:household_id) { login_data.dig('household', 'id') }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')) }
  let(:source) { schedules(:john_movicol) }

  def path(action) = "/api/v1/households/#{household_id}/schedules/#{source.id}/dose_occurrences/#{action}"

  def record_not_taken
    headers
    post path('not_taken'), params: { dose_occurrence: { key: occurrence_key, reason: 'unwell', note: 'Resting' } },
                            headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    response.parsed_body.fetch('data')
  end

  def occurrence_key
    MedicationAdministration::OccurrenceProjection.new(
      source: source.reload, start_date: Date.current, end_date: Date.current
    ).call.first.key
  end

  it 'reopens using the current version and retains attributable former context' do
    row = record_not_taken
    patch path('reopen'), params: { dose_occurrence: { key: row.fetch('key') } },
                          headers: headers.merge('If-Match' => row.fetch('etag')), as: :json
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('data')).to include('outcome' => 'open', 'reason' => nil, 'note' => nil)
    record = source.medication_dose_occurrences.sole
    expect(record.versions.last.reify).to have_attributes(reason: 'unwell', note: 'Resting')
  end

  it 'requires a current version for correction' do
    row = record_not_taken
    [nil, 'stale-version'].each do |etag|
      patch path('reopen'), params: { dose_occurrence: { key: row.fetch('key') } },
                            headers: headers.merge('If-Match' => etag), as: :json
      expect(response).to have_http_status(etag ? :conflict : :precondition_required)
      expect(source.medication_dose_occurrences.sole.outcome).to eq('not_taken')
    end
  end

  it 'requires manage access to reopen' do
    row = record_not_taken
    membership = users(:admin).person.account.first_active_household_membership
    PersonAccessGrant.where(household_membership: membership, person_id: source.person_id).find_each do |grant|
      grant.update!(access_level: 'record')
    end
    patch path('reopen'), params: { dose_occurrence: { key: row.fetch('key') } },
                          headers: headers.merge('If-Match' => row.fetch('etag')), as: :json
    expect(response).to have_http_status(:forbidden)
  end

  it 'requires the observed version before replacing a not-taken decision' do
    row = record_not_taken
    [nil, 'stale-version'].each do |etag|
      post path('take'), params: { dose_occurrence: { key: row.fetch('key'), taken_at: Time.current.iso8601 } },
                         headers: headers.merge('If-Match' => etag), as: :json
      expect(response).to have_http_status(etag ? :conflict : :precondition_required)
      expect(source.medication_dose_occurrences.sole).to be_not_taken
    end
  end

  it 'replaces not-taken with one immutable take and one stock decrement' do
    row = record_not_taken
    source.medication.update!(current_supply: 100)
    attributes = { key: row.fetch('key'), taken_at: Time.current.iso8601, client_uuid: SecureRandom.uuid }
    expect do
      post path('take'), params: { dose_occurrence: attributes },
                         headers: headers.merge('If-Match' => row.fetch('etag')), as: :json
    end.to change(MedicationTake, :count).by(1)
    expect(response).to have_http_status(:ok)
    record = source.medication_dose_occurrences.sole
    expect(record).to be_taken
    supply = source.medication.reload.current_supply
    expect(supply).to be < 100
    expect(record.versions.last.reify.note).to eq('Resting')
    post path('take'), params: { dose_occurrence: attributes }, headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(source.medication.reload.current_supply).to eq(supply)
    patch path('reopen'), params: { dose_occurrence: { key: row.fetch('key') } },
                          headers: headers.merge('If-Match' => Api::RecordEtag.for(record)), as: :json
    expect(response).to have_http_status(:unprocessable_content)
    expect(record.reload).to be_taken
  end

  it 'rejects an invalid administration time without changing history or stock' do
    row = record_not_taken
    expect do
      post path('take'), params: { dose_occurrence: { key: row.fetch('key'), taken_at: 'invalid' } },
                         headers: headers, as: :json
    end.not_to change(MedicationTake, :count)
    expect(response).to have_http_status(:unprocessable_content)
    expect(source.medication_dose_occurrences.sole).to be_not_taken
  end
end
