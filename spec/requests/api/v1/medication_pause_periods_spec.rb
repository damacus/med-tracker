require 'rails_helper'

RSpec.describe 'API v1 medication pause periods' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules,
           :person_medications

  let(:login_data) { api_login(users(:admin)) }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')) }
  let(:source) { schedules(:john_paracetamol) }
  let(:path) { "/api/v1/households/#{household_id}/medication_pause_periods" }
  let(:attributes) do
    { source_type: 'schedule', source_id: source.portable_id, reason: 'out_of_supply', note: 'Delivery tomorrow' }
  end

  def household_id
    login_data.dig('household', 'id')
  end

  def create_pause(values = attributes, request_headers = headers)
    post path, params: { medication_pause_period: values }, headers: request_headers, as: :json
  end

  it 'creates attributable context at server acceptance and ignores client timestamps' do
    freeze_time do
      create_pause(attributes.merge(started_at: 1.year.ago.iso8601))
      expect(response).to have_http_status(:created)
      data = response.parsed_body.fetch('data')
      expect(data).to include(
        'source_id' => source.portable_id, 'source_type' => 'schedule',
        'reason' => 'out_of_supply', 'note' => 'Delivery tomorrow',
        'started_at' => Time.current.iso8601, 'ended_at' => nil
      )
      expect(data.fetch('recorded_by_membership_id')).to be_a(String)
      expect(data.fetch('recorded_by_name')).to be_present
      expect(source.reload).to be_paused
    end
  end

  it 'supports direct assignments' do
    assignment = person_medications(:john_vitamin_d)
    create_pause(attributes.merge(source_type: 'person_medication', source_id: assignment.portable_id))
    expect(response).to have_http_status(:created)
    expect(response.parsed_body.dig('data', 'source_id')).to eq(assignment.portable_id)
    expect(assignment.reload).to be_paused
  end

  [nil, '', 'reason_not_recorded', 'unsupported'].each do |reason|
    it "rejects public reason #{reason.inspect}" do
      create_pause(attributes.merge(reason: reason))
      expect(response).to have_http_status(:unprocessable_content)
      expect(source.reload).not_to be_paused
    end
  end

  it 'replays one committed pause for an idempotency key' do
    request_headers = headers.merge('Idempotency-Key' => SecureRandom.uuid)
    create_pause(attributes, request_headers)
    first_body = response.parsed_body
    create_pause(attributes, request_headers)
    expect(response.parsed_body).to eq(first_body)
    expect(response.headers['Idempotency-Replayed']).to eq('true')
    expect(source.medication_pause_periods.count).to eq(1)
  end

  %w[create resume].each do |action|
    it "does not replay #{action} context after the person grant is revoked" do
      delegated_headers, grant = delegated_manager
      request_headers = delegated_headers.merge('Idempotency-Key' => SecureRandom.uuid)
      create_pause(attributes, action == 'create' ? request_headers : delegated_headers)
      period_id = response.parsed_body.dig('data', 'id')
      post "#{path}/#{period_id}/resume", headers: request_headers, as: :json if action == 'resume'
      expect(response).to have_http_status(action == 'create' ? :created : :ok)
      grant.update!(revoked_at: Time.current)

      if action == 'create'
        create_pause(attributes, request_headers)
      else
        post "#{path}/#{period_id}/resume", headers: request_headers, as: :json
      end

      expect(response).to have_http_status(:not_found)
      expect(response.parsed_body.to_s).not_to include('Delivery tomorrow', 'out_of_supply')
      expect(response.headers['Idempotency-Replayed']).to be_nil
      expect(source.medication_pause_periods.count).to eq(1)
    end
  end

  it 'does not replay legacy pause context after the person grant is revoked' do
    delegated_headers, grant = delegated_manager
    create_pause(attributes, delegated_headers)
    request_headers = delegated_headers.merge('Idempotency-Key' => SecureRandom.uuid)
    legacy_path = "/api/v1/households/#{household_id}/schedules/#{source.portable_id}/pause"
    patch legacy_path, headers: request_headers, as: :json
    expect(response.parsed_body.dig('data', 'current_pause_period', 'note')).to eq('Delivery tomorrow')
    grant.update!(revoked_at: Time.current)

    patch legacy_path, headers: request_headers, as: :json

    expect(response).to have_http_status(:not_found)
    expect(response.parsed_body.to_s).not_to include('Delivery tomorrow', 'out_of_supply')
    expect(response.headers['Idempotency-Replayed']).to be_nil
  end

  it 'lists only the selected source history' do
    create_pause
    get path, params: { source_type: 'schedule', source_id: source.portable_id }, headers: headers
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('data').pluck('source_id')).to eq([source.portable_id])
    expect(response.parsed_body.dig('meta', 'total_count')).to eq(1)
  end

  it 'paginates history with the newest pause first' do
    create_pause
    old_id = response.parsed_body.dig('data', 'id')
    post "#{path}/#{old_id}/resume", headers: headers, as: :json
    create_pause(attributes.merge(reason: 'other'))
    newest_id = response.parsed_body.dig('data', 'id')

    get path, params: { source_type: 'schedule', source_id: source.portable_id, per_page: 1 }, headers: headers

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('data').pluck('id')).to eq([newest_id])
    expect(response.parsed_body.dig('meta', 'total_count')).to eq(2)
  end

  it 'resumes the addressed period without changing its original context' do
    create_pause
    period_id = response.parsed_body.dig('data', 'id')
    post "#{path}/#{period_id}/resume", headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('data')).to include('reason' => 'out_of_supply', 'note' => 'Delivery tomorrow')
    expect(response.parsed_body.dig('data', 'ended_at')).to be_present
    expect(response.parsed_body.dig('data', 'resumed_by_name')).to be_present
    expect(source.reload).not_to be_paused
  end

  it 'does not close a newer pause when resuming a completed period again' do
    create_pause
    old_id = response.parsed_body.dig('data', 'id')
    post "#{path}/#{old_id}/resume", headers: headers, as: :json
    create_pause(attributes.merge(reason: 'other'))
    new_id = response.parsed_body.dig('data', 'id')
    post "#{path}/#{old_id}/resume", headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'id')).to eq(old_id)
    expect(source.reload).to be_paused
    expect(MedicationPausePeriod.find_by!(portable_id: new_id).ended_at).to be_nil
  end

  it 'returns not found for a hidden source without exposing context' do
    create_pause(attributes.merge(source_id: SecureRandom.uuid))
    expect(response).to have_http_status(:not_found)
    expect(response.parsed_body.to_s).not_to include('Delivery tomorrow')
  end

  it 'does not expose another household pause through history or resume' do
    other_household = create(:household)
    other_source = create(:schedule, household: other_household,
                                     person: create(:person, household: other_household),
                                     medication: create(:medication, household: other_household))
    other_source.pause!
    period = other_source.medication_pause_periods.sole

    get path, params: { source_type: 'schedule', source_id: other_source.portable_id }, headers: headers
    expect(response).to have_http_status(:not_found)
    post "#{path}/#{period.portable_id}/resume", headers: headers, as: :json
    expect(response).to have_http_status(:not_found)
    expect(period.reload.ended_at).to be_nil
  end

  it 'allows viewing history but rejects writes without manage access' do
    create_pause
    period_id = response.parsed_body.dig('data', 'id')
    viewer = users(:jane)
    owner = HouseholdMembership.find_by!(account: users(:admin).person.account, household_id: household_id)
    membership = HouseholdMembership.find_by!(household_id: household_id, account: viewer.person.account)
    membership.person_access_grants.where(person: source.person).delete_all
    PersonAccessGrant.create!(household_id: household_id, household_membership: membership,
                              person: source.person, access_level: :view, relationship_type: :family_member,
                              granted_by_membership: owner)
    viewer_login = api_login(viewer, household_id: household_id)
    viewer_headers = api_auth_headers(viewer_login.fetch('access_token'))

    get path, headers: viewer_headers
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('data').pluck('id')).to include(period_id)
    create_pause(attributes, viewer_headers)
    expect(response).to have_http_status(:forbidden)
    post "#{path}/#{period_id}/resume", headers: viewer_headers, as: :json
    expect(response).to have_http_status(:forbidden)
    expect(source.reload).to be_paused
    expect(source.medication_pause_periods.sole.ended_at).to be_nil
  end

  %w[schedule person_medication].each do |source_type|
    it "retains #{source_type} history after retirement while rejecting new writes" do
      record = source_type == 'schedule' ? source : person_medications(:john_vitamin_d)
      values = attributes.merge(source_type: source_type, source_id: record.portable_id)
      create_pause(values)
      period_id = response.parsed_body.dig('data', 'id')
      record.retire!

      get path, params: values.slice(:source_type, :source_id), headers: headers
      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.fetch('data').pluck('id')).to eq([period_id])
      get path, headers: headers
      expect(response.parsed_body.fetch('data').pluck('id')).to include(period_id)
      create_pause(values)
      expect(response).to have_http_status(:not_found)
      post "#{path}/#{period_id}/resume", headers: headers, as: :json
      expect(response).to have_http_status(:not_found)
      expect(record.reload).to be_paused
    end
  end

  it 'retains the legacy pause operation and adds current context' do
    patch "/api/v1/households/#{household_id}/schedules/#{source.portable_id}/pause", headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'current_pause_period', 'reason')).to eq('reason_not_recorded')
  end

  def delegated_manager
    actor_login = api_login(users(:jane), household_id: household_id)
    membership = HouseholdMembership.find_by!(household_id: household_id, account: users(:jane).person.account)
    [api_auth_headers(actor_login.fetch('access_token')), grant_manage_access(membership)]
  end

  def grant_manage_access(membership)
    grant = membership.person_access_grants.find_or_initialize_by(person: source.person)
    grant.update!(household_id: household_id, access_level: :manage, relationship_type: :family_member,
                  granted_by_membership: users(:admin).person.household_membership)
    grant
  end

  it 'advertises the reason-required operation' do
    get api_v1_capabilities_path, as: :json
    expect(response.parsed_body.dig('data', 'medication_pause_periods')).to include(
      'supported' => true, 'reasons' => MedicationPausePeriod::PUBLIC_REASONS, 'effective_time' => 'server_acceptance'
    )
  end
end
