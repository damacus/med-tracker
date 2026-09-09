require 'rails_helper'

RSpec.describe 'API v1 offline replay authority' do
  fixtures :all

  let(:login) { api_login(users(:admin)) }
  let(:household_id) { login.dig('household', 'id') }
  let(:actor) { api_login(users(:jane), household_id: household_id) }
  let(:headers) { api_auth_headers(actor.fetch('access_token')).merge('Idempotency-Key' => SecureRandom.uuid) }
  let(:grant) do
    membership = ApiSession.lookup_by_access_token(actor.fetch('access_token')).household_membership
    record = PersonAccessGrant.find_or_initialize_by(household_id: household_id, person: people(:john),
                                                     household_membership: membership, revoked_at: nil)
    record.tap { |row| row.update!(access_level: :manage, relationship_type: :family_member) }
  end

  def post_batch(*operations, request_headers: headers)
    post api_v1_household_sync_batches_path(household_id),
         params: { batch: { operations: operations } }, headers: request_headers, as: :json
  end

  def operation(record, resource_type, action: 'update', attributes: {})
    { resource_type: resource_type, action: action, id: record.portable_id,
      if_match: Api::RecordEtag.for(record), attributes: attributes }
  end

  it 'advertises the exact supported action matrix and the online-only boundary' do
    get '/api/v1/capabilities', as: :json
    sync = response.parsed_body.dig('data', 'sync')
    matrix = sync.fetch('operations').to_h { |row| [row.fetch('resource_type'), row.fetch('actions')] }
    expect(matrix).to eq(
      'medication_take' => %w[create], 'medication_dose_occurrence' => %w[create update],
      'medication_pause_period' => %w[create close],
      'medication' => %w[create update delete adjust_inventory mark_as_ordered mark_as_received remove_stock],
      'medication_dosage_option' => %w[create update], 'person' => %w[create update],
      'health_event' => %w[create update delete], 'location' => %w[create update delete],
      'medication_review_prompt' => %w[update], 'schedule' => %w[create update delete pause resume],
      'person_medication' => %w[create update delete pause resume reorder]
    )
    expect(sync.fetch('online_only_resources')).to include('profile', 'avatar', 'invitation', 'household_membership',
                                                           'person_access_grant', 'location_membership', 'report')
  end

  it 'rejects every online-only operation with the same code before any writes' do
    admin_headers = api_auth_headers(login.fetch('access_token'))
    %w[profile avatar invitation household_membership person_access_grant location_membership report
       unknown].each do |type|
      %w[create update delete].each do |action|
        post_batch({ resource_type: 'location', action: 'create', attributes: { name: 'Must roll back' } },
                   { resource_type: type, action: action }, request_headers: admin_headers)
        expect(response).to have_http_status(:unprocessable_content)
        expect(response.parsed_body.dig('error', 'code')).to eq('sync_operation_unsupported')
        expect(Location.where(name: 'Must roll back')).not_to exist
      end
    end
  end

  it 'rechecks a person grant before returning a cached update' do
    grant
    change = operation(people(:john), 'person', attributes: { name: 'Private offline update' })
    post_batch(change)
    expect(response).to have_http_status(:created)
    grant.update!(revoked_at: Time.current)
    post_batch(change)
    expect(response).to have_http_status(:forbidden)
    expect(response.parsed_body['data']).to be_nil
  end

  it 'rechecks medicine visibility before returning a cached creation' do
    grant
    change = { resource_type: 'medication', action: 'create', attributes: {
      name: 'Queued medicine', current_supply: '10', dose_amount: '1', dose_unit: 'tablet',
      location_id: locations(:home).portable_id
    } }
    post_batch(change)
    expect(response).to have_http_status(:created)
    medicine = Medication.find_by!(portable_id: response.parsed_body.dig('data', 'results', 0, 'record_portable_id'))
    post_batch(change)
    expect(response).to have_http_status(:created)
    expect(response.headers['Idempotency-Replayed']).to eq('true')
    private_person = Person.create!(household_id: household_id, name: 'Private adult',
                                    date_of_birth: 40.years.ago.to_date, person_type: :adult)
    create(:schedule, household_id: household_id, person: private_person, medication: medicine)
    get api_v1_household_medication_path(household_id, medicine), headers: headers, as: :json
    expect(response).to have_http_status(:not_found)

    post_batch(change)

    expect(response).to have_http_status(:forbidden)
    expect(response.parsed_body['data']).to be_nil
  end

  it 'rechecks current grant strength before returning a cached assignment action' do
    grant
    change = operation(schedules(:john_paracetamol), 'schedule', action: 'pause')
    post_batch(change)
    expect(response).to have_http_status(:created)
    grant.update!(access_level: :view)
    post_batch(change)
    expect(response).to have_http_status(:forbidden)
    expect(response.parsed_body['data']).to be_nil
  end

  it 'replays a retired assignment only while its current management grant remains' do
    grant
    change = operation(schedules(:john_paracetamol), 'schedule', action: 'delete')
    post_batch(change)
    expect(response).to have_http_status(:created)
    original = response.parsed_body
    post_batch(change)
    expect(response.parsed_body).to eq(original)
    grant.update!(revoked_at: Time.current)
    post_batch(change)
    expect(response).to have_http_status(:forbidden)
  end

  it 'replays a deleted health event through its tombstone only while access remains' do
    grant
    event = HealthEvent.create!(household_id: household_id, person: people(:john), title: 'Private illness',
                                event_kind: :illness, severity: :mild, started_on: Date.current)
    change = operation(event, 'health_event', action: 'delete')
    post_batch(change)
    expect(response).to have_http_status(:created)
    original = response.parsed_body
    post_batch(change)
    expect(response.parsed_body).to eq(original)
    grant.update!(revoked_at: Time.current)
    post_batch(change)
    expect(response).to have_http_status(:forbidden)
    expect(response.parsed_body['data']).to be_nil
  end

  it 'does not reconstruct a missing retired assignment from its tombstone' do
    grant
    source = create(:schedule, household_id: household_id, person: people(:john), medication: medications(:ibuprofen))
    change = operation(source, 'schedule', action: 'delete')
    post_batch(change)
    expect(response).to have_http_status(:created)
    expect(ApiTombstone.where(record_portable_id: source.portable_id)).to exist
    source.reload.delete
    post_batch(change)
    expect(response).to have_http_status(:not_found)
    expect(response.parsed_body['data']).to be_nil
  end

  it 'rolls back mixed pause and inventory effects when a later version conflicts' do
    household_id
    source = schedules(:john_paracetamol)
    medication = medications(:ibuprofen)
    before_state = [source.active, medication.current_supply]
    counts = [MedicationPausePeriod, PaperTrail::Version, ApiChangeEvent, ApiTombstone].map(&:count)
    post_batch(operation(source, 'schedule', action: 'pause'),
               operation(medication, 'medication', action: 'adjust_inventory', attributes: { new_quantity: '20' }),
               operation(people(:john), 'person', attributes: { name: 'Conflict' }).merge(if_match: 'stale'),
               request_headers: api_auth_headers(login.fetch('access_token')))
    expect(response).to have_http_status(:conflict)
    expect([source.reload.active, medication.reload.current_supply]).to eq(before_state)
    expect([MedicationPausePeriod, PaperTrail::Version, ApiChangeEvent, ApiTombstone].map(&:count)).to eq(counts)
  end

  it 'does not replay a deleted health event when its older tombstone cannot establish person access' do
    grant
    event = HealthEvent.create!(household_id: household_id, person: people(:john), title: 'Earlier illness',
                                event_kind: :illness, severity: :mild, started_on: Date.current)
    change = operation(event, 'health_event', action: 'delete')
    post_batch(change)
    expect(response).to have_http_status(:created)
    ApiTombstone.find_by!(record_portable_id: event.portable_id).update!(metadata: {})
    post_batch(change)
    expect(response).to have_http_status(:not_found)
    expect(response.parsed_body['data']).to be_nil
  end

  it 'rechecks medicine visibility before returning a cached order' do
    grant
    medicine = create(:medication, household_id: household_id, location: locations(:home))
    create(:schedule, household_id: household_id, person: people(:john), medication: medicine)
    change = operation(medicine, 'medication', action: 'mark_as_ordered')
    post_batch(change)
    expect(response).to have_http_status(:created)
    grant.update!(revoked_at: Time.current)
    post_batch(change)
    expect(response).to have_http_status(:forbidden)
    expect(response.parsed_body['data']).to be_nil
  end

  it 'replays a deleted location using its household tombstone' do
    household_id
    record = Location.create!(household_id: household_id, name: 'Temporary cupboard')
    change = operation(record, 'location', action: 'delete')
    key_headers = api_auth_headers(login.fetch('access_token')).merge('Idempotency-Key' => SecureRandom.uuid)
    post_batch(change, request_headers: key_headers)
    expect(response).to have_http_status(:created)
    original = response.parsed_body
    post_batch(change, request_headers: key_headers)
    expect(response).to have_http_status(:created)
    expect(response.parsed_body).to eq(original)
    expect(response.headers['Idempotency-Replayed']).to eq('true')
  end

  it 'does not return a cached update after its record has been deleted' do
    household_id
    record = Location.create!(household_id: household_id, name: 'Temporary cupboard')
    change = operation(record, 'location', attributes: { description: 'Changed offline' })
    key_headers = api_auth_headers(login.fetch('access_token')).merge('Idempotency-Key' => SecureRandom.uuid)
    post_batch(change, request_headers: key_headers)
    expect(response).to have_http_status(:created)
    record.reload.destroy!
    post_batch(change, request_headers: key_headers)
    expect(response).to have_http_status(:not_found)
    expect(response.parsed_body['data']).to be_nil
  end

  it 'rechecks medicine visibility before returning a cached receipt' do
    grant
    medicine = create(:medication, household_id: household_id, location: locations(:home))
    create(:schedule, household_id: household_id, person: people(:john), medication: medicine)
    change = operation(medicine, 'medication', action: 'mark_as_received')
    post_batch(change)
    expect(response).to have_http_status(:created)
    original = response.parsed_body
    post_batch(change)
    expect(response.parsed_body).to eq(original)
    grant.update!(revoked_at: Time.current)
    post_batch(change)
    expect(response).to have_http_status(:forbidden)
    expect(response.parsed_body['data']).to be_nil
  end

  it 'replays a saved validation failure without treating it as a successful resource result' do
    household_id
    medicine = medications(:ibuprofen)
    supply = medicine.current_supply
    key_headers = api_auth_headers(login.fetch('access_token')).merge('Idempotency-Key' => SecureRandom.uuid)
    change = operation(medicine, 'medication', action: 'adjust_inventory', attributes: { new_quantity: '-1' })
    post_batch(change, request_headers: key_headers)
    expect(response).to have_http_status(:unprocessable_content)
    original = response.parsed_body
    post_batch(change, request_headers: key_headers)
    expect(response.parsed_body).to eq(original)
    expect(response.headers['Idempotency-Replayed']).to eq('true')
    expect(medicine.reload.current_supply).to eq(supply)
  end
end
