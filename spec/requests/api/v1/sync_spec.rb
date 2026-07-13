# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 sync' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

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
end
