# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 sync' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules,
           :person_medications, :medication_takes, :carer_relationships

  let(:user) { users(:admin) }
  let(:login_data) { api_login(user) }
  let(:household_id) { login_data.dig('household', 'id') }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')) }

  it 'returns a portable v2 snapshot without sensitive platform records' do
    event = HealthEvent.create!(
      household_id: household_id,
      person: people(:john),
      event_kind: :illness,
      title: 'Snapshot cold',
      started_on: '2026-02-25'
    )

    get api_v1_household_sync_snapshot_path(household_id), headers: headers, as: :json

    expect(response).to have_http_status(:ok)
    data = response.parsed_body.fetch('data')
    expect(data.fetch('format')).to eq('medtracker.portable.v2')
    expect(data.fetch('records')).to include('people', 'medications', 'schedules', 'health_events')
    expect(data.dig('records', 'health_events')).to contain_exactly(
      include(
        'portable_id' => event.portable_id,
        'person_portable_id' => event.person.portable_id,
        'title' => 'Snapshot cold',
        'etag' => be_present
      )
    )
    expect(data.fetch('records')).not_to include('api_sessions', 'api_app_tokens', 'native_device_tokens',
                                                 'push_subscriptions', 'household_invitations',
                                                 'security_audit_events')
    expect(data.fetch('cursor')).to be_present
  end

  it 'records model changes and tombstones outside API controllers' do
    event = HealthEvent.create!(
      household_id: household_id,
      person: people(:john),
      event_kind: :illness,
      title: 'Web-originated cold',
      started_on: '2026-02-25'
    )
    session = ApiSession.lookup_by_access_token(login_data.fetch('access_token'))

    TenantContext.with(
      account: session.account,
      household: session.household_membership.household,
      membership: session.household_membership,
      request_id: 'web-request'
    ) do
      expect { event.update!(title: 'Web-originated recovery') }
        .to change { ApiChangeEvent.where(record_type: 'HealthEvent', action: 'update').count }.by(1)
      expect { event.destroy! }
        .to change { ApiTombstone.where(record_type: 'HealthEvent', action: 'delete').count }.by(1)
    end
  end

  it 'records changes to mobile-visible relationship collections' do
    event = HealthEvent.create!(
      household_id: household_id,
      person: people(:john),
      event_kind: :illness,
      title: 'Relationship sync cold',
      started_on: '2026-02-25'
    )
    session = ApiSession.lookup_by_access_token(login_data.fetch('access_token'))
    household = session.household_membership.household

    TenantContext.with(
      account: session.account,
      household: household,
      membership: session.household_membership,
      request_id: 'relationship-request'
    ) do
      location = Location.create!(household: household, name: 'Relationship sync location')

      expect { LocationMembership.create!(household: household, location: location, person: people(:john)) }
        .to change { ApiChangeEvent.where(record_type: 'Person', action: 'update').count }.by(1)
      expect do
        HealthEventMedication.create!(
          household: household,
          health_event: event,
          medication: medications(:paracetamol)
        )
      end.to change { ApiChangeEvent.where(record_type: 'HealthEvent', action: 'update').count }.by(1)
    end
  end

  it 'returns cursor changes and tombstones' do
    cursor = 5.minutes.ago.iso8601
    medication = medications(:paracetamol)
    ApiChangeEvent.create!(
      household_id: household_id,
      account: user.person.account,
      household_membership: ApiSession.lookup_by_access_token(login_data.fetch('access_token')).household_membership,
      record_type: 'Medication',
      record_id: medication.id,
      record_portable_id: medication.portable_id,
      action: 'update',
      occurred_at: 1.minute.ago,
      metadata: { record_type: 'Medication' }
    )
    ApiTombstone.create!(
      household_id: household_id,
      account: user.person.account,
      household_membership: ApiSession.lookup_by_access_token(login_data.fetch('access_token')).household_membership,
      record_type: 'HealthEvent',
      record_portable_id: SecureRandom.uuid,
      deleted_at: Time.current
    )

    get api_v1_household_sync_changes_path(household_id),
        params: { cursor: cursor },
        headers: headers,
        as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'changes').first).to include('record_type' => 'Medication')
    expect(response.parsed_body.dig('data', 'tombstones').first).to include('record_type' => 'HealthEvent')
  end

  it 'applies batch mutations transactionally and rolls back invalid operations' do
    medication = medications(:paracetamol)
    original_name = medication.name

    post api_v1_household_sync_batches_path(household_id),
         params: {
           batch: {
             operations: [
               {
                 action: 'update',
                 resource_type: 'medication',
                 id: medication.portable_id,
                 if_match: Api::RecordEtag.for(medication),
                 attributes: { name: 'Batch Updated Paracetamol' }
               },
               {
                 action: 'update',
                 resource_type: 'unsupported_resource',
                 id: medications(:ibuprofen).portable_id,
                 attributes: { name: 'Invalid' }
               }
             ]
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:unprocessable_content)
    expect(medication.reload.name).to eq(original_name)
  end

  it 'requires a version precondition for batch updates' do
    medication = medications(:paracetamol)

    post api_v1_household_sync_batches_path(household_id),
         params: {
           batch: {
             operations: [
               {
                 action: 'update',
                 resource_type: 'medication',
                 id: medication.portable_id,
                 attributes: { name: 'Unsafe update' }
               }
             ]
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:precondition_required)
    expect(response.parsed_body.dig('error', 'code')).to eq('precondition_required')
    expect(medication.reload.name).not_to eq('Unsafe update')
  end

  it 'rejects stale batch versions with a machine-readable conflict' do
    medication = medications(:paracetamol)

    post api_v1_household_sync_batches_path(household_id),
         params: {
           batch: {
             operations: [
               {
                 action: 'update',
                 resource_type: 'medication',
                 id: medication.portable_id,
                 if_match: '"stale-etag"',
                 attributes: { name: 'Stale update' }
               }
             ]
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:conflict)
    expect(response.parsed_body.dig('error', 'code')).to eq('sync_conflict')
    expect(medication.reload.name).not_to eq('Stale update')
  end

  it 'rechecks the version after acquiring the mutation lock' do
    medication = medications(:paracetamol)
    original_etag = Api::RecordEtag.for(medication)
    locked_record = Medication.find(medication.id)
    locator = instance_double(Api::PortableRecordLocator, find: locked_record)
    allow(Api::PortableRecordLocator).to receive(:new).and_return(locator)
    allow(locked_record).to receive(:with_lock).and_wrap_original do |method, *args, &block|
      medication.update!(name: 'Concurrent web update')
      method.call(*args, &block)
    end

    post api_v1_household_sync_batches_path(household_id),
         params: {
           batch: {
             operations: [
               {
                 action: 'update',
                 resource_type: 'medication',
                 id: medication.portable_id,
                 if_match: original_etag,
                 attributes: { name: 'Unsafe native overwrite' }
               }
             ]
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:conflict)
    expect(medication.reload.name).not_to eq('Unsafe native overwrite')
  end

  it 'records tombstones for batch deletes' do
    event = HealthEvent.create!(
      household_id: household_id,
      person: people(:john),
      event_kind: :illness,
      title: 'Resolved cold',
      started_on: '2026-02-25'
    )

    expect do
      post api_v1_household_sync_batches_path(household_id),
           params: {
             batch: {
               operations: [
                 {
                   action: 'delete',
                   resource_type: 'health_event',
                   id: event.portable_id,
                   if_match: Api::RecordEtag.for(event),
                   attributes: {}
                 }
               ]
             }
           },
           headers: headers,
           as: :json
    end.to change(ApiTombstone, :count).by(1)

    expect(response).to have_http_status(:created)
    expect(HealthEvent.exists?(event.id)).to be(false)
  end

  it 'deletes medications without administration history in batch mutations' do
    household = Household.find(household_id)
    medication = create(:medication, household: household, location: locations(:home))

    expect do
      post api_v1_household_sync_batches_path(household_id),
           params: {
             batch: {
               operations: [
                 {
                   action: 'delete',
                   resource_type: 'medication',
                   id: medication.portable_id,
                   if_match: Api::RecordEtag.for(medication),
                   attributes: {}
                 }
               ]
             }
           },
           headers: headers,
           as: :json
    end.to change(ApiTombstone, :count).by(1)

    expect(response).to have_http_status(:created)
    expect(Medication.exists?(medication.id)).to be(false)
  end

  it 'rejects medication batch deletion when administration history must be retained' do
    household = Household.find(household_id)
    medication = create(:medication, household: household, location: locations(:home))
    schedule = create(:schedule, household: household, person: people(:john), medication: medication)
    take = create(:medication_take, :for_schedule, household: household, schedule: schedule)

    expect do
      post api_v1_household_sync_batches_path(household_id),
           params: {
             batch: {
               operations: [
                 {
                   action: 'delete',
                   resource_type: 'medication',
                   id: medication.portable_id,
                   if_match: Api::RecordEtag.for(medication),
                   attributes: {}
                 }
               ]
             }
           },
           headers: headers,
           as: :json
    end.not_to change(ApiTombstone, :count)

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.parsed_body.fetch('error')).to include(
      'code' => 'unprocessable_content',
      'message' => 'operation 0 delete conflicts with retained administration history'
    )
    expect(Medication.exists?(medication.id)).to be(true)
    expect(Schedule.exists?(schedule.id)).to be(true)
    expect(MedicationTake.exists?(take.id)).to be(true)
  end

  it 'updates health events in batch mutations' do
    event = HealthEvent.create!(
      household_id: household_id,
      person: people(:john),
      event_kind: :illness,
      title: 'Cold',
      started_on: '2026-02-25'
    )

    post api_v1_household_sync_batches_path(household_id),
         params: {
           batch: {
             operations: [
               {
                 action: 'update',
                 resource_type: 'health_event',
                 id: event.portable_id,
                 if_match: Api::RecordEtag.for(event),
                 attributes: { title: 'Recovered cold', severity: 'mild', ended_on: '2026-02-26' }
               }
             ]
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:created)
    expect(event.reload).to have_attributes(title: 'Recovered cold', severity: 'mild', ended_on: Date.new(2026, 2, 26))
  end

  it 'rejects unsupported batch actions before writing records' do
    medication = medications(:paracetamol)

    post api_v1_household_sync_batches_path(household_id),
         params: {
           batch: {
             operations: [
               {
                 action: 'replace',
                 resource_type: 'medication',
                 id: medication.portable_id,
                 attributes: { name: 'Unsupported Replace' }
               }
             ]
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:unprocessable_content)
    expect(medication.reload.name).not_to eq('Unsupported Replace')
  end

  it 'rejects create operations for unsupported resources before writing records' do
    expect do
      post_batch(
        {
          action: 'create',
          resource_type: 'medication',
          attributes: { name: 'Unsupported Create' }
        }
      )
    end.not_to change(Medication, :count)

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.parsed_body.fetch('error')).to include('code' => 'sync_operation_unsupported')
  end

  it 'rolls back batch updates with invalid attributes' do
    event = HealthEvent.create!(
      household_id: household_id,
      person: people(:john),
      event_kind: :illness,
      title: 'Cold',
      started_on: '2026-02-25'
    )

    post api_v1_household_sync_batches_path(household_id),
         params: {
           batch: {
             operations: [
               {
                 action: 'update',
                 resource_type: 'health_event',
                 id: event.portable_id,
                 if_match: Api::RecordEtag.for(event),
                 attributes: { title: '', ended_on: '2026-02-24' }
               }
             ]
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:unprocessable_content)
    expect(event.reload.title).to eq('Cold')
  end

  describe 'medication-take create operations' do
    before { travel_to(2.days.from_now) }

    it 'rejects future queued doses without creating a take' do
      expect do
        post_batch(medication_take_operation(source: schedules(:jane_ibuprofen), taken_at: 2.days.from_now.iso8601))
      end.not_to change(MedicationTake, :count)

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body.fetch('error')).to include('code' => 'medication_take_invalid')
    end

    it 'records a queued schedule take through the batch result contract' do
      source = schedules(:jane_ibuprofen)
      client_uuid = SecureRandom.uuid

      expect do
        post_batch(medication_take_operation(source: source, client_uuid: client_uuid))
      end.to change(MedicationTake, :count).by(1)

      expect(response).to have_http_status(:created)
      take = MedicationTake.find_by!(client_uuid: client_uuid)
      expect(take).to have_attributes(schedule: source, household_id: household_id)
      expect(response.parsed_body.dig('data', 'results', 0)).to include(
        'action' => 'create',
        'record_type' => 'MedicationTake',
        'record_portable_id' => take.portable_id,
        'replayed' => false
      )
    end

    it 'records a queued person-medication take' do
      source = person_medications(:jane_vitamin_d)
      client_uuid = SecureRandom.uuid

      post_batch(medication_take_operation(source: source, client_uuid: client_uuid))

      expect(response).to have_http_status(:created)
      expect(MedicationTake.find_by!(client_uuid: client_uuid)).to have_attributes(person_medication: source)
    end

    it 'requires a non-blank client UUID' do
      source = schedules(:jane_ibuprofen)

      expect do
        post_batch(medication_take_operation(source: source, client_uuid: ''))
      end.not_to change(MedicationTake, :count)

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body.fetch('error')).to include(
        'code' => 'medication_take_invalid',
        'message' => 'operation 0 client_uuid is required'
      )
    end

    it 'rejects update and delete actions for immutable medication takes' do
      take = medication_takes(:jane_morning_ibuprofen)

      %w[update delete].each do |action|
        post_batch(
          {
            action: action,
            resource_type: 'medication_take',
            id: take.portable_id,
            if_match: Api::RecordEtag.for(take),
            attributes: {}
          }
        )

        expect(response).to have_http_status(:unprocessable_content)
        expect(response.parsed_body.fetch('error')).to include('code' => 'sync_operation_unsupported')
        expect(response.parsed_body.to_json).not_to include(take.portable_id)
      end

      expect(take.reload).to eq(take)
    end

    it 'replays an authorized take without repeating stock, audit, or sync side effects' do
      take = medication_takes(:jane_morning_ibuprofen)
      medication = take.schedule.medication
      counts = medication_take_side_effect_counts(take)
      supply = medication.current_supply

      post_batch(
        medication_take_operation(
          source: take.schedule,
          client_uuid: take.client_uuid,
          taken_at: 3.days.from_now.iso8601
        )
      )

      expect(response).to have_http_status(:created)
      expect(response.parsed_body.dig('data', 'results', 0)).to include(
        'record_portable_id' => take.portable_id,
        'replayed' => true
      )
      expect(medication.reload.current_supply).to eq(supply)
      expect(medication_take_side_effect_counts(take)).to eq(counts)
    end

    it 'requires record access after resolving a visible source' do
      scoped_user = users(:jane)
      scoped_login = api_login(scoped_user)
      scoped_household_id = scoped_login.dig('household', 'id')
      membership = scoped_user.person.account.household_memberships.find_by!(household_id: scoped_household_id)
      grant = PersonAccessGrant.find_by!(household_membership: membership, person: scoped_user.person)
      grant.update!(access_level: :view)
      source = schedules(:jane_ibuprofen)

      expect do
        post_batch(
          medication_take_operation(source: source),
          request_headers: api_auth_headers(scoped_login.fetch('access_token')),
          target_household_id: scoped_household_id
        )
      end.not_to change(MedicationTake, :count)

      expect(response).to have_http_status(:forbidden)
      expect(response.parsed_body.dig('error', 'code')).to eq('forbidden')
    end

    it 'hides stale and cross-household source references' do
      stale_id = SecureRandom.uuid
      other_household = create(:household)
      other_person = create(:person, household: other_household)
      other_medication = create(:medication, household: other_household)
      other_source = create(
        :schedule,
        household: other_household,
        person: other_person,
        medication: other_medication
      )

      [stale_id, other_source.portable_id].each do |source_id|
        operation = medication_take_operation(source: schedules(:jane_ibuprofen))
        operation[:attributes][:source_id] = source_id
        post_batch(operation)

        expect(response).to have_http_status(:not_found)
        expect(response.parsed_body.fetch('error')).to include(
          'code' => 'not_found',
          'message' => 'Record not found'
        )
        expect(response.parsed_body.to_json).not_to include(source_id)
      end
    end

    it 'returns stable errors for unavailable stock and timing conflicts' do
      household = Household.find(household_id)
      empty_medication = create(
        :medication,
        household: household,
        current_supply: 0,
        supply_at_last_restock: 0
      )
      empty_source = create(
        :schedule,
        household: household,
        person: people(:jane),
        medication: empty_medication
      )
      timing_source = schedules(:jane_ibuprofen)
      timing_take = medication_takes(:jane_morning_ibuprofen)

      [
        [
          medication_take_operation(
            source: empty_source,
            taken_from_medication_id: empty_medication.id
          ),
          'medication_stock_unavailable'
        ],
        [
          medication_take_operation(source: timing_source, taken_at: (timing_take.taken_at + 1.hour).iso8601),
          'medication_timing_conflict'
        ]
      ].each do |operation, code|
        post_batch(operation)

        expect(response).to have_http_status(:unprocessable_content)
        expect(response.parsed_body.dig('error', 'code')).to eq(code)
        expect(response.parsed_body.to_json).not_to include(
          operation.dig(:attributes, :client_uuid),
          operation.dig(:attributes, :source_id)
        )
      end
    end

    it 'rejects invalid dose input without echoing clinical values' do
      private_value = 'private-dose-time'

      post_batch(
        medication_take_operation(
          source: schedules(:jane_ibuprofen),
          taken_at: private_value,
          dose_amount: 'private-dose-value'
        )
      )

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body.dig('error', 'code')).to eq('medication_take_invalid')
      expect(response.parsed_body.to_json).not_to include(private_value, 'private-dose-value')
    end

    it 'rejects unsupported source types without echoing the source reference' do
      operation = medication_take_operation(source: schedules(:jane_ibuprofen))
      private_source_type = 'private-source-type'
      private_source_id = 'private-source-id'
      operation[:attributes].merge!(source_type: private_source_type, source_id: private_source_id)

      post_batch(operation)

      expect(response).to have_http_status(:not_found)
      expect(response.parsed_body.fetch('error')).to include('code' => 'not_found', 'message' => 'Record not found')
      expect(response.parsed_body.to_json).not_to include(private_source_type, private_source_id)
    end

    it 'returns a stable validation error after retrying an unrelated persistence failure' do
      dose_service = instance_double(MedicationAdministration::RecordDose)
      result = MedicationAdministration::RecordDose::Result.new(success: false, take: nil, error: :create_failed)
      allow(MedicationAdministration::RecordDose).to receive(:new).and_return(dose_service)
      allow(dose_service).to receive(:call).and_return(result)

      expect do
        post_batch(medication_take_operation(source: schedules(:jane_ibuprofen)))
      end.not_to change(MedicationTake, :count)

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body.fetch('error')).to include(
        'code' => 'medication_take_invalid',
        'message' => 'Medication take is invalid'
      )
      expect(dose_service).to have_received(:call).twice
    end

    it 'rolls back an earlier update when a queued take fails' do
      medication = medications(:paracetamol)
      original_name = medication.name
      original_changes = ApiChangeEvent.count

      post_batch(
        {
          action: 'update',
          resource_type: 'medication',
          id: medication.portable_id,
          if_match: Api::RecordEtag.for(medication),
          attributes: { name: 'Must Roll Back' }
        },
        medication_take_operation(source: schedules(:jane_ibuprofen), client_uuid: '')
      )

      expect(response).to have_http_status(:unprocessable_content)
      expect(medication.reload.name).to eq(original_name)
      expect(ApiChangeEvent.count).to eq(original_changes)
    end

    it 'rolls back a queued take and all side effects when a later operation fails' do
      source = schedules(:jane_ibuprofen)
      medication = source.medication
      client_uuid = SecureRandom.uuid
      initial = {
        takes: MedicationTake.count,
        supply: medication.current_supply,
        versions: PaperTrail::Version.where(item_type: 'MedicationTake').count,
        changes: ApiChangeEvent.count,
        tombstones: ApiTombstone.count
      }

      post_batch(
        medication_take_operation(
          source: source,
          client_uuid: client_uuid,
          taken_at: Time.current.iso8601
        ),
        {
          action: 'replace',
          resource_type: 'medication',
          id: medications(:paracetamol).portable_id,
          attributes: { name: 'Unsupported' }
        }
      )

      expect(response).to have_http_status(:unprocessable_content)
      expect(
        takes: MedicationTake.count,
        supply: medication.reload.current_supply,
        versions: PaperTrail::Version.where(item_type: 'MedicationTake').count,
        changes: ApiChangeEvent.count,
        tombstones: ApiTombstone.count
      ).to eq(initial)
      expect(MedicationTake.exists?(client_uuid: client_uuid)).to be(false)
    end

    it 'does not disclose an idempotency key that cannot be replayed' do
      hidden_take = medication_takes(:jane_morning_ibuprofen)
      hidden_uuid = hidden_take.client_uuid
      scoped_login = api_login(users(:carer))
      scoped_household_id = scoped_login.dig('household', 'id')

      post_batch(
        medication_take_operation(source: schedules(:patient_schedule), client_uuid: hidden_uuid),
        request_headers: api_auth_headers(scoped_login.fetch('access_token')),
        target_household_id: scoped_household_id
      )

      expect(response).to have_http_status(:conflict)
      expect(response.parsed_body.dig('error', 'code')).to eq('idempotency_key_unavailable')
      expect(response.parsed_body.to_json).not_to include(
        hidden_uuid,
        hidden_take.portable_id,
        hidden_take.person.portable_id
      )
    end
  end

  def post_batch(*operations, request_headers: headers, target_household_id: household_id)
    post api_v1_household_sync_batches_path(target_household_id),
         params: { batch: { operations: operations } },
         headers: request_headers,
         as: :json
  end

  def medication_take_operation(
    source:, client_uuid: SecureRandom.uuid, taken_at: Time.current.iso8601, **attributes
  )
    source_type = source.is_a?(Schedule) ? 'schedule' : 'person_medication'
    {
      action: 'create',
      resource_type: 'medication_take',
      attributes: {
        client_uuid: client_uuid,
        source_type: source_type,
        source_id: source.portable_id,
        taken_at: taken_at
      }.merge(attributes)
    }
  end

  def medication_take_side_effect_counts(take)
    {
      takes: MedicationTake.where(client_uuid: take.client_uuid).count,
      versions: take.versions.count,
      changes: ApiChangeEvent.where(record_type: 'MedicationTake', record_id: take.id).count
    }
  end
end
