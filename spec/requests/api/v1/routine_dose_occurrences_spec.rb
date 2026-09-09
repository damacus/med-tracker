require 'rails_helper'

RSpec.describe 'API v1 routine dose occurrences' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages

  let(:login_data) { api_login(users(:admin)) }
  let(:household_id) { login_data.dig('household', 'id') }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')).merge('Idempotency-Key' => SecureRandom.uuid) }
  let(:source) do
    create(:person_medication, :routine, person: people(:john), medication: medications(:vitamin_c),
                                         max_daily_doses: 1, created_at: 2.months.ago)
  end

  def path = "/api/v1/households/#{household_id}/person_medications/#{source.id}/dose_occurrences"
  def range = { start_date: Date.current.iso8601, end_date: Date.current.iso8601 }

  def occurrence_key
    MedicationAdministration::OccurrenceProjection.new(source: source.reload, start_date: Date.current,
                                                       end_date: Date.current).call.first.key
  end

  def submit_not_taken(key: occurrence_key)
    post "#{path}/not_taken", params: { dose_occurrence: { key: key, reason: 'unwell' } }, headers: headers, as: :json
  end

  %w[daily weekly monthly].each do |cycle|
    it "reads a stable #{cycle} occurrence with its inclusive cycle end" do
      source.update!(dose_cycle: cycle)
      expect { get path, params: range, headers: headers }.not_to change(MedicationDoseOccurrence, :count)
      expect(response).to have_http_status(:ok)
      window = DoseCycle.new(cycle).range_for(Time.current)
      expect(response.parsed_body.fetch('data').sole).to include(
        'source_type' => 'person_medication', 'source_id' => source.id, 'scheduled_at' => nil,
        'window_starts_on' => window.begin.to_date.iso8601, 'window_ends_on' => window.end.to_date.iso8601
      )
    end
  end

  it 'records and replays a not-taken decision without stock or take changes' do
    stock = source.medication.current_supply
    expect { submit_not_taken }.not_to change(MedicationTake, :count)
    expect(response).to have_http_status(:ok)
    expect { submit_not_taken }.not_to change(PaperTrail::Version, :count)
    expect(response.headers['Idempotency-Replayed']).to eq('true')
    expect(source.medication.reload.current_supply).to eq(stock)
  end

  it 'reopens an outcome only with its current version' do
    submit_not_taken
    etag = response.headers['ETag']
    headers['Idempotency-Key'] = SecureRandom.uuid
    patch "#{path}/reopen", params: { dose_occurrence: { key: occurrence_key } }, headers: headers, as: :json
    expect(response).to have_http_status(:precondition_required)
    headers['Idempotency-Key'] = SecureRandom.uuid
    patch "#{path}/reopen", params: { dose_occurrence: { key: occurrence_key } },
                            headers: headers.merge('If-Match' => etag), as: :json
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'outcome')).to eq('open')
  end

  it 'links one monthly administration and deducts stock once on replay' do
    source.update!(dose_cycle: :monthly)
    source.medication.update!(current_supply: 100)
    payload = { dose_occurrence: { key: occurrence_key, taken_at: Time.current.iso8601,
                                   client_uuid: SecureRandom.uuid } }
    post "#{path}/take", params: payload, headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'outcome')).to eq('taken')
    expect { post "#{path}/take", params: payload, headers: headers, as: :json }.not_to change(MedicationTake, :count)
    expect(source.medication.reload.current_supply).to eq(99)
  end

  it 'does not expose as-needed expectations or accept a former routine key' do
    key = occurrence_key
    source.update!(administration_kind: :as_needed)
    get path, params: range, headers: headers
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('data')).to be_empty
    submit_not_taken(key: key)
    expect(response).to have_http_status(:unprocessable_content)
  end

  it 'checks person access before replaying a cached decision' do
    submit_not_taken
    actor = users(:admin).person.account.first_active_household_membership
    PersonAccessGrant.where(household_membership: actor, person_id: source.person_id).find_each do |grant|
      grant.update!(revoked_at: Time.current)
    end
    submit_not_taken
    expect(response).to have_http_status(:not_found)
    expect(response.body).not_to include('unwell')
  end

  it 'hides a routine source belonging to another household' do
    other_person = create(:person, household: create(:household))
    other = create(:person_medication, :routine, person: other_person)
    get path.sub("/#{source.id}/dose_occurrences", "/#{other.id}/dose_occurrences"), params: range, headers: headers
    expect(response).to have_http_status(:not_found)
  end
end
