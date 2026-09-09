require 'rails_helper'

RSpec.shared_examples 'queued outcome contract' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  let(:login_data) { api_login(users(:admin)) }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')) }

  def household_id = login_data.dig('household', 'id')

  def not_taken_operation
    key = MedicationAdministration::OccurrenceProjection.new(
      source: source.reload, start_date: Date.current, end_date: Date.current
    ).call.first.key
    { action: 'create', resource_type: 'medication_dose_occurrence', attributes: {
      source_type: MedicationDoseSource.new(source).type, source_id: source.portable_id, occurrence_key: key,
      outcome: 'not_taken', reason: 'unwell', note: 'Queued decision'
    } }
  end

  def reopen_operation(record, etag: Api::RecordEtag.for(record))
    { action: 'update', resource_type: 'medication_dose_occurrence', id: record.portable_id,
      if_match: etag, attributes: { outcome: 'open' } }
  end

  def submit(operations, request_headers: headers)
    post "/api/v1/households/#{household_id}/sync/batches", params: { batch: { operations: operations } },
                                                            headers: request_headers, as: :json
  end

  def resolve_outcome
    submit([not_taken_operation])
    expect(response).to have_http_status(:created)
    source.medication_dose_occurrences.sole
  end

  it 'records and replays a decision without a take or stock change' do
    headers
    operation = not_taken_operation
    supply = source.medication.current_supply
    expect { submit([operation]) }.not_to change(MedicationTake, :count)
    expect(response).to have_http_status(:created)
    record = source.medication_dose_occurrences.sole
    expect(record).to be_not_taken
    expect(response.parsed_body.dig('data', 'results', 0, 'etag')).to eq(Api::RecordEtag.for(record))
    expect { submit([operation]) }.not_to change(MedicationDoseOccurrence, :count)
    expect(response).to have_http_status(:created)
    expect(source.medication.reload.current_supply).to eq(supply)
  end

  it 'rejects a conflicting replay without changing the original reason' do
    record = resolve_outcome
    operation = not_taken_operation
    operation[:attributes][:reason] = 'refused'
    submit([operation])
    expect(response).to have_http_status(:conflict)
    expect(record.reload.reason).to eq('unwell')
  end

  it 'replays a cached outcome batch without duplicating the decision' do
    change = not_taken_operation
    key_headers = headers.merge('Idempotency-Key' => SecureRandom.uuid)
    submit([change], request_headers: key_headers)
    expect(response).to have_http_status(:created)
    original = response.parsed_body
    expect { submit([change], request_headers: key_headers) }.not_to change(MedicationDoseOccurrence, :count)
    expect(response).to have_http_status(:created)
    expect(response.parsed_body).to eq(original)
    expect(response.headers['Idempotency-Replayed']).to eq('true')
  end

  it 'replays an omitted reason sent as an empty string' do
    operation = not_taken_operation
    operation[:attributes][:reason] = ''
    submit([operation])
    expect(response).to have_http_status(:created)
    submit([operation])
    expect(response).to have_http_status(:created)
    expect(source.medication_dose_occurrences.sole.reason).to be_nil
  end

  it 'reopens the identified decision with its current version' do
    record = resolve_outcome
    submit([reopen_operation(record)])
    expect(response).to have_http_status(:created)
    expect(record.reload).to be_open
    expect(record.note).to be_nil
    expect(response.parsed_body.dig('data', 'results', 0, 'etag')).to eq(Api::RecordEtag.for(record))
  end

  it 'rejects a missing or stale version before reopening' do
    record = resolve_outcome
    submit([reopen_operation(record, etag: nil)])
    expect(response).to have_http_status(:precondition_required)
    submit([reopen_operation(record, etag: 'stale')])
    expect(response).to have_http_status(:conflict)
    expect(record.reload).to be_not_taken
  end

  it 'rolls back earlier writes when an outcome is invalid' do
    headers
    source.reload
    original_notes = source.notes
    update = { action: 'update', resource_type: MedicationDoseSource.new(source).type, id: source.portable_id,
               if_match: Api::RecordEtag.for(source), attributes: { notes: 'Queued edit' } }
    operation = not_taken_operation
    operation[:attributes][:occurrence_key] = 'invalid'
    submit([update, operation])
    expect(response).to have_http_status(:unprocessable_content)
    expect(source.reload.notes).to eq(original_notes)
    expect(source.medication_dose_occurrences).to be_empty
  end

  it 'rolls back a resolved outcome when a later operation fails' do
    submit([not_taken_operation, { action: 'create', resource_type: 'unsupported' }])
    expect(response).to have_http_status(:unprocessable_content)
    expect(source.medication_dose_occurrences).to be_empty
  end

  it 'does not allow deleting saved decisions' do
    record = resolve_outcome
    submit([reopen_operation(record).merge(action: 'delete')])
    expect(response).to have_http_status(:unprocessable_content)
    expect(record.reload).to be_not_taken
  end

  it 'rechecks access before replaying a cached batch response' do
    request_headers = headers.merge('Idempotency-Key' => SecureRandom.uuid)
    operation = not_taken_operation
    submit([operation], request_headers: request_headers)
    expect(response).to have_http_status(:created)
    membership = users(:admin).person.account.first_active_household_membership
    PersonAccessGrant.where(household_membership: membership, person_id: source.person_id).find_each do |grant|
      grant.update!(access_level: :view)
    end
    submit([operation], request_headers: request_headers)
    expect(response).to have_http_status(:forbidden)
  end

  it 'allows record access to resolve an outcome but requires manage access to reopen it' do
    headers
    membership = users(:admin).person.account.first_active_household_membership
    PersonAccessGrant.where(household_membership: membership, person_id: source.person_id).find_each do |grant|
      grant.update!(access_level: :record)
    end
    record = resolve_outcome
    submit([reopen_operation(record)])
    expect(response).to have_http_status(:forbidden)
    expect(record.reload).to be_not_taken
  end

  it 'rejects invalid clinical fields without returning their values' do
    operation = not_taken_operation
    operation[:attributes][:reason] = 'private-invalid-reason'
    submit([operation])
    expect(response).to have_http_status(:unprocessable_content)
    expect(response.body).not_to include('private-invalid-reason', 'Queued decision')
    expect(source.medication_dose_occurrences).to be_empty
  end
end

RSpec.describe 'API v1 queued dose outcomes' do
  context 'with a formal schedule' do
    let(:source) { schedules(:john_movicol) }

    it_behaves_like 'queued outcome contract'
  end

  context 'with a routine assignment' do
    let(:source) do
      create(:person_medication, :routine, person: people(:john), medication: medications(:vitamin_c),
                                           dose_cycle: :monthly, max_daily_doses: 1, created_at: 2.months.ago)
    end

    it_behaves_like 'queued outcome contract' do
      it 'rejects a routine key after the source becomes as-needed' do
        operation = not_taken_operation
        source.update!(administration_kind: :as_needed)
        submit([operation])
        expect(response).to have_http_status(:unprocessable_content)
        expect(source.medication_dose_occurrences).to be_empty
      end
    end
  end
end
