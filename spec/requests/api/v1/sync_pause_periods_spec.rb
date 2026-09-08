require 'rails_helper'

RSpec.describe 'API v1 pause-period sync' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules,
           :person_medications

  let(:login_data) { api_login(users(:admin)) }
  let(:household_id) { login_data.dig('household', 'id') }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')) }
  let(:membership) do
    ApiSession.lookup_by_access_token(login_data.fetch('access_token')).household_membership
  end
  let(:schedule) { schedules(:john_paracetamol) }

  describe 'snapshot and change feeds' do
    it 'includes pause periods visible through the member person grants' do
      visible = create_period(schedule)
      schedule.update!(retired_at: Time.current)
      hidden_source = create(:schedule, household_id: household_id, person: people(:bob),
                                        medication: medications(:ibuprofen))
      hidden = create_period(hidden_source)
      restricted = restricted_access_for(people(:john))

      get api_v1_household_sync_snapshot_path(household_id), headers: restricted, as: :json

      periods = response.parsed_body.dig('data', 'records', 'medication_pause_periods')
      expect(response).to have_http_status(:ok)
      expect(periods.pluck('portable_id')).to include(visible.portable_id)
      expect(periods.pluck('portable_id')).not_to include(hidden.portable_id)
    end

    it 'returns visible pause-period changes without disclosing hidden or foreign records' do
      visible = create_period(schedule)
      schedule.update!(retired_at: Time.current)
      hidden_source = create(:schedule, household_id: household_id, person: people(:bob),
                                        medication: medications(:ibuprofen))
      hidden = create_period(hidden_source)
      foreign_source = create(:schedule)
      foreign_source.update!(active: false)
      foreign = foreign_source.medication_pause_periods.create!(
        reason: MedicationPausePeriod::LEGACY_REASON, legacy_context: true
      )
      [visible, hidden, foreign].each do |period|
        ApiChangeEvent.create!(household: period.household, record_type: period.class.name, record_id: period.id,
                               record_portable_id: period.portable_id, action: 'create', occurred_at: Time.current)
      end
      restricted = restricted_access_for(people(:john))

      get api_v1_household_sync_changes_path(household_id),
          params: { cursor: 1.minute.ago.iso8601 },
          headers: restricted,
          as: :json

      ids = response.parsed_body.dig('data', 'changes').pluck('record_portable_id')
      expect(response).to have_http_status(:ok)
      expect(ids).to include(visible.portable_id)
      expect(ids).not_to include(hidden.portable_id, foreign.portable_id)
    end
  end

  describe 'batch create' do
    it 'pauses schedules and direct assignments with public context at server time' do
      assignment = person_medications(:john_vitamin_d)

      freeze_time do
        post_batch(
          create_operation(schedule),
          create_operation(assignment, note: nil)
        )

        expect(response).to have_http_status(:created)
        [schedule, assignment].each do |source|
          period = source.medication_pause_periods.sole
          expect(period).to have_attributes(reason: 'out_of_supply', started_at: Time.current,
                                            recorded_by_membership: membership)
          expect(source.reload).to be_paused
        end
        expect(response.parsed_body.dig('data', 'results')).to all(
          include('action' => 'create', 'record_type' => 'MedicationPausePeriod',
                  'etag' => be_present, 'replayed' => false)
        )
      end
    end

    [nil, '', 'reason_not_recorded', 'unsupported'].each do |reason|
      it "rejects public reason #{reason.inspect}" do
        post_batch(create_operation(schedule, reason: reason))

        expect(response).to have_http_status(:unprocessable_content)
        expect(schedule.reload).not_to be_paused
        expect(schedule.medication_pause_periods).to be_empty
      end
    end

    it 'replays an existing pause without replacing its context' do
      post_batch(create_operation(schedule))
      original = schedule.medication_pause_periods.sole.attributes

      post_batch(create_operation(schedule, reason: 'other', note: 'Replacement'))

      expect(response).to have_http_status(:created)
      expect(response.parsed_body.dig('data', 'results', 0, 'replayed')).to be(true)
      expect(schedule.medication_pause_periods.sole.attributes).to eq(original)
    end

    it 'rejects client effective times and actor fields' do
      %i[started_at ended_at recorded_by_membership_id resumed_by_membership_id].each do |field|
        post_batch(create_operation(schedule, field => 1.year.ago.iso8601))
        expect(response).to have_http_status(:unprocessable_content)
      end

      expect(schedule.reload).not_to be_paused
      expect(schedule.medication_pause_periods).to be_empty
    end

    it 'requires a portable source UUID' do
      post_batch(create_operation(schedule).tap { |operation| operation[:attributes][:source_id] = schedule.id.to_s })

      expect(response).to have_http_status(:not_found)
      expect(schedule.reload).not_to be_paused
    end

    it 'replays the committed batch response for an idempotency key' do
      retry_headers = headers.merge('Idempotency-Key' => SecureRandom.uuid)
      operation = create_operation(schedule)
      post_batch(operation, request_headers: retry_headers)
      original = response.parsed_body
      counts = side_effect_counts

      post_batch(operation, request_headers: retry_headers)

      expect(response.parsed_body).to eq(original)
      expect(response.headers['Idempotency-Replayed']).to eq('true')
      expect(side_effect_counts).to eq(counts)
    end

    it 'does not replay a success after the member loses source access' do
      access = restricted_access(people(:john))
      retry_headers = access.fetch(:headers).merge('Idempotency-Key' => SecureRandom.uuid)
      operation = create_operation(schedule)
      post_batch(operation, request_headers: retry_headers)
      expect(response).to have_http_status(:created)

      access.fetch(:grant).update!(revoked_at: Time.current)
      post_batch(operation, request_headers: retry_headers)

      expect(response).to have_http_status(:not_found)
      expect(response.headers['Idempotency-Replayed']).not_to eq('true')
    end
  end

  describe 'batch close' do
    it 'closes the addressed period through the lifecycle service' do
      period = create_period(schedule)

      post_batch(close_operation(period))

      expect(response).to have_http_status(:created)
      expect(period.reload).to have_attributes(ended_at: be_present, resumed_by_membership: membership)
      expect(schedule.reload).not_to be_paused
      expect(response.parsed_body.dig('data', 'results', 0)).to include(
        'action' => 'close', 'record_portable_id' => period.portable_id,
        'etag' => Api::RecordEtag.for(period), 'replayed' => false
      )
    end

    it 'requires a fresh period ETag' do
      period = create_period(schedule)
      post_batch(close_operation(period).except(:if_match))
      expect(response).to have_http_status(:precondition_required)
      expect(period.reload.ended_at).to be_nil

      post_batch(close_operation(period).merge(if_match: '"stale"'))
      expect(response).to have_http_status(:conflict)
      expect(response.parsed_body.dig('error', 'code')).to eq('sync_conflict')
      expect(period.reload.ended_at).to be_nil
      expect(schedule.reload).to be_paused
    end

    it 'replays a completed close without closing a newer pause' do
      old_period = create_period(schedule)
      post_batch(close_operation(old_period))
      post_batch(create_operation(schedule, reason: 'other'))
      newer_period = schedule.medication_pause_periods.find_by!(ended_at: nil)

      post_batch(close_operation(old_period.reload))

      expect(response).to have_http_status(:created)
      expect(response.parsed_body.dig('data', 'results', 0, 'replayed')).to be(true)
      expect(newer_period.reload.ended_at).to be_nil
      expect(schedule.reload).to be_paused
    end
  end

  it 'rejects correction and deletion actions without changing a period' do
    period = create_period(schedule)
    original = period.attributes

    %w[update delete].each do |action|
      post_batch({ action: action, resource_type: 'medication_pause_period', id: period.portable_id,
                   if_match: Api::RecordEtag.for(period), attributes: { reason: 'other', note: 'Changed' } })
      expect(response).to have_http_status(:unprocessable_content)
    end

    expect(period.reload.attributes).to eq(original)
  end

  it 'rejects context fields on close' do
    period = create_period(schedule)

    post_batch(close_operation(period).merge(attributes: { reason: 'other' }))

    expect(response).to have_http_status(:unprocessable_content)
    expect(period.reload.ended_at).to be_nil
    expect(schedule.reload).to be_paused
  end

  it 'rolls back source, period, audit and sync writes when a later operation fails' do
    original_counts = side_effect_counts

    post_batch(
      create_operation(schedule),
      { action: 'update', resource_type: 'unsupported', id: SecureRandom.uuid, attributes: {} }
    )

    expect(response).to have_http_status(:unprocessable_content)
    expect(schedule.reload).not_to be_paused
    expect(schedule.medication_pause_periods).to be_empty
    expect(side_effect_counts).to eq(original_counts)
  end

  private

  def create_period(source)
    source.update!(active: false)
    source.medication_pause_periods.create!(reason: 'out_of_supply', note: 'Delivery tomorrow',
                                            started_at: 1.hour.ago, recorded_by_membership: membership)
  end

  def create_operation(source, reason: 'out_of_supply', note: 'Delivery tomorrow', **extra)
    source_type = source.is_a?(Schedule) ? 'schedule' : 'person_medication'
    { action: 'create', resource_type: 'medication_pause_period', attributes: {
      source_type: source_type, source_id: source.portable_id, reason: reason, note: note
    }.merge(extra) }
  end

  def close_operation(period)
    { action: 'close', resource_type: 'medication_pause_period', id: period.portable_id,
      if_match: Api::RecordEtag.for(period), attributes: {} }
  end

  def post_batch(*operations, request_headers: headers)
    post api_v1_household_sync_batches_path(household_id),
         params: { batch: { operations: operations } }, headers: request_headers, as: :json
  end

  def restricted_access_for(person)
    restricted_access(person).fetch(:headers)
  end

  def restricted_access(person)
    data = api_login(users(:doctor), household_id: household_id)
    actor_membership = ApiSession.lookup_by_access_token(data.fetch('access_token')).household_membership
    PersonAccessGrant.where(household_membership: actor_membership).find_each do |grant|
      grant.update!(revoked_at: Time.current)
    end
    grant = PersonAccessGrant.create!(household_id: household_id, household_membership: actor_membership,
                                      person: person, access_level: :manage, relationship_type: :professional)
    { headers: api_auth_headers(data.fetch('access_token')), grant: grant }
  end

  def side_effect_counts
    period_versions = PaperTrail::Version.where(item_type: 'MedicationPausePeriod')
    source_versions = PaperTrail::Version.where(item_type: 'Schedule', item_id: schedule.id)
    {
      MedicationPausePeriod => MedicationPausePeriod.count,
      PaperTrail::Version => period_versions.or(source_versions).count,
      ApiChangeEvent => ApiChangeEvent.where(record_type: %w[MedicationPausePeriod Schedule]).count
    }
  end
end
