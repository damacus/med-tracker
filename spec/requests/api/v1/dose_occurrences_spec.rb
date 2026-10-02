require 'rails_helper'

RSpec.describe 'API v1 dose occurrences' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  let(:login_data) { api_login(users(:admin)) }
  let(:household_id) { login_data.dig('household', 'id') }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')) }
  let(:source) { schedules(:john_movicol) }
  let(:date) { Date.current }

  def path = "/api/v1/households/#{household_id}/schedules/#{source.id}/dose_occurrences"
  def range = { start_date: date.iso8601, end_date: date.iso8601 }

  it 'returns stable untimed identities without writing occurrence rows' do
    expect { get path, params: range, headers: headers }.not_to change(MedicationDoseOccurrence, :count)
    expect(response).to have_http_status(:ok)
    row = response.parsed_body.fetch('data').sole
    expect(row).to include('source_type' => 'schedule', 'source_id' => source.id,
                           'window_starts_on' => date.iso8601, 'position' => 1, 'scheduled_at' => nil,
                           'outcome' => 'open', 'expected' => true)
    get path, params: range, headers: headers
    expect(response.parsed_body.fetch('data').sole.fetch('key')).to eq(row.fetch('key'))
  end

  it 'returns distinct timed occurrences' do
    source.update!(schedule_config: { 'times' => %w[08:00 20:00] })
    get path, params: range, headers: headers
    expect(response).to have_http_status(:ok)
    rows = response.parsed_body.fetch('data')
    expect(rows.map { |row| row.fetch('key') }.uniq.size).to eq(2)
    expect(rows.map { |row| row.fetch('scheduled_at') }).to all(be_present)
  end

  it 'does not project PRN sources' do
    source.update!(frequency: 'As needed')
    get path, params: range, headers: headers
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('data')).to be_empty
  end

  it 'rejects missing, malformed, reversed and oversized date ranges' do
    [{}, range.merge(start_date: 'private clinical text'), range.merge(end_date: Date.yesterday.iso8601),
     range.merge(end_date: (date + 31).iso8601)].each do |parameters|
      get path, params: parameters, headers: headers
      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body.dig('error', 'message')).not_to include('private clinical text')
    end
  end

  it 'hides a source from another household' do
    other = create(:household)
    person = create(:person, household: other)
    medication = create(:medication, household: other)
    other_source = create(:schedule, household: other, person: person, medication: medication)
    get "/api/v1/households/#{household_id}/schedules/#{other_source.id}/dose_occurrences",
        params: range, headers: headers
    expect(response).to have_http_status(:not_found)
  end

  it 'requires authentication' do
    get path, params: range
    expect(response).to have_http_status(:unauthorized)
  end

  it 'requires a current person-level grant even for a household owner' do
    headers
    membership = users(:admin).person.account.first_active_household_membership
    PersonAccessGrant.where(household_membership: membership, person_id: source.person_id).find_each do |grant|
      grant.update!(revoked_at: Time.current)
    end

    get path, params: range, headers: headers
    expect(response).to have_http_status(:not_found)
  end

  it 'returns persisted context and its correction version' do
    headers
    source.reload
    membership = users(:admin).person.account.first_active_household_membership
    record = source.medication_dose_occurrences.create!(
      window_starts_on: date, position: 1, outcome: 'not_taken', reason: 'unwell', note: 'Resting',
      resolved_at: Time.current, resolved_by_membership: membership
    )

    get path, params: range, headers: headers
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('data').sole).to include(
      'outcome' => 'not_taken', 'reason' => 'unwell', 'note' => 'Resting', 'etag' => Api::RecordEtag.for(record)
    )
  end
end
