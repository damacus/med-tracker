require 'rails_helper'

RSpec.shared_examples 'outcome sync read contract' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  let(:login_data) { api_login(users(:admin)) }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')) }
  let(:session) { ApiSession.lookup_by_access_token(login_data.fetch('access_token')) }
  let(:membership) { session.household_membership }

  def household_id = login_data.dig('household', 'id')
  def cursor = 5.minutes.ago.iso8601

  def in_context(&)
    TenantContext.with(account: session.account, household: membership.household, membership: membership, &)
  end

  def record_outcome
    headers
    in_context do
      source.medication_dose_occurrences.create!(window_starts_on: Date.current, position: 1, outcome: 'not_taken',
                                                 reason: 'unwell', note: 'Resting', resolved_at: Time.current,
                                                 resolved_by_membership: membership)
    end
  end

  def changes
    get api_v1_household_sync_changes_path(household_id), params: { cursor: cursor }, headers: headers
    response.parsed_body.fetch('data')
  end

  def revoke_access
    PersonAccessGrant.where(household_membership: membership, person_id: source.person_id)
                     .find_each { |grant| grant.update!(revoked_at: Time.current) }
  end

  it 'includes stored outcomes in a consistent portable snapshot without projecting new rows' do
    outcome = record_outcome
    expect do
      get api_v1_household_sync_snapshot_path(household_id), headers: headers
    end.not_to change(MedicationDoseOccurrence, :count)
    expect(response).to have_http_status(:ok)
    payload = response.parsed_body.fetch('data')
    expect(payload.dig('records', 'dose_occurrences')).to contain_exactly(
      include('portable_id' => outcome.portable_id, 'outcome' => 'not_taken', 'note' => 'Resting')
    )
    expect(payload.fetch('cursor')).to be_present
  end

  it 'exposes the current outcome and version for changes made outside API controllers' do
    outcome = record_outcome
    event = changes.fetch('changes').find { |row| row['record_type'] == 'MedicationDoseOccurrence' }
    expect(event).to include('record_portable_id' => outcome.portable_id)
    expect(event.fetch('record')).to include('outcome' => 'not_taken', 'etag' => Api::RecordEtag.for(outcome))
    expect(event.fetch('metadata').to_s).not_to include('Resting', 'unwell')
  end

  it 'does not expose outcomes, source records or events after a person grant is revoked' do
    outcome = record_outcome
    revoke_access
    expect(changes.fetch('changes').pluck('record_portable_id')).not_to include(outcome.portable_id)
    get api_v1_household_sync_snapshot_path(household_id), headers: headers
    records = response.parsed_body.dig('data', 'records')
    expect(records.fetch('dose_occurrences')).to be_empty
    collection = MedicationDoseSource.new(source).type.pluralize
    expect(records.fetch(collection).pluck('portable_id')).not_to include(source.portable_id)
  end

  it 'returns the latest saved state when an outcome has changed more than once' do
    outcome = record_outcome
    in_context do
      outcome.update!(outcome: 'open', reason: nil, note: nil, resolved_at: nil, resolved_by_membership: nil)
    end
    events = changes.fetch('changes').select { |row| row['record_portable_id'] == outcome.portable_id }
    expect(events.pluck('action')).to eq(%w[create update])
    expected_record = { 'outcome' => 'open', 'note' => nil, 'etag' => Api::RecordEtag.for(outcome) }
    expect(events.pluck('record')).to all(include(expected_record))
  end

  it 'keeps outcome tombstones visible only while access to the person remains current' do
    outcome = record_outcome
    in_context { outcome.destroy! }
    expect(changes.fetch('tombstones').pluck('record_portable_id')).to include(outcome.portable_id)
    revoke_access
    expect(changes.fetch('tombstones').pluck('record_portable_id')).not_to include(outcome.portable_id)
  end

  it 'includes outcomes and their source for a member with a view grant' do
    outcome = record_outcome
    delegate_login = api_login(users(:carer))
    delegate = ApiSession.lookup_by_access_token(delegate_login.fetch('access_token')).household_membership
    grant = PersonAccessGrant.find_or_initialize_by(household_membership: delegate, person: source.person)
    grant.update!(household: membership.household, access_level: :view, relationship_type: :family_member)

    get api_v1_household_sync_snapshot_path(household_id),
        headers: api_auth_headers(delegate_login.fetch('access_token'))
    expect(response).to have_http_status(:ok)
    records = response.parsed_body.dig('data', 'records')
    expect(records.fetch('dose_occurrences').pluck('portable_id')).to include(outcome.portable_id)
    collection = MedicationDoseSource.new(source).type.pluralize
    expect(records.fetch(collection).pluck('portable_id')).to include(source.portable_id)
  end
end

RSpec.describe 'API v1 dose outcome sync reads' do
  context 'with a formal schedule' do
    let(:source) { schedules(:john_movicol).reload }

    it_behaves_like 'outcome sync read contract'
  end

  context 'with a routine assignment' do
    let(:source) do
      create(:person_medication, :routine, person: people(:john), medication: medications(:vitamin_c),
                                           max_daily_doses: 1, created_at: 2.months.ago)
    end

    it_behaves_like 'outcome sync read contract'
  end
end
