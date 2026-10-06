require 'rails_helper'

RSpec.shared_examples 'queued occurrence take contract' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  let(:login_data) { api_login(users(:admin)) }
  let(:household_id) { login_data.dig('household', 'id') }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')) }
  let(:client_uuid) { SecureRandom.uuid }

  def occurrence_key
    MedicationAdministration::OccurrenceProjection.new(
      source: source.reload, start_date: Date.current, end_date: Date.current
    ).call.first.key
  end

  def take_operation(key: occurrence_key)
    { action: 'create', resource_type: 'medication_take', attributes: {
      client_uuid: client_uuid, source_type: MedicationDoseSource.new(source).type, source_id: source.portable_id,
      taken_at: Time.current.iso8601, occurrence_key: key
    } }
  end

  def submit(operations)
    post "/api/v1/households/#{household_id}/sync/batches", params: { batch: { operations: operations } },
                                                            headers: headers, as: :json
  end

  it 'atomically links a queued dose and replays it without another administration' do
    headers
    source.medication.update!(current_supply: 100)
    operation = take_operation
    expect { submit([operation]) }.to change(MedicationTake, :count).by(1)
    expect(response).to have_http_status(:created)
    record = source.medication_dose_occurrences.sole
    expect(record).to be_taken
    expect(record.medication_take.client_uuid).to eq(client_uuid)
    supply = source.medication.reload.current_supply
    expect(supply).to be < 100
    expect { submit([operation]) }.not_to change(MedicationTake, :count)
    expect(response).to have_http_status(:created)
    expect(source.medication.reload.current_supply).to eq(supply)
  end

  it 'rejects a mismatched key and rolls back earlier changes in the batch' do
    headers
    source.reload
    original_notes = source.notes
    update = { action: 'update', resource_type: MedicationDoseSource.new(source).type, id: source.portable_id,
               if_match: Api::RecordEtag.for(source), attributes: { notes: 'Queued edit' } }
    expect { submit([update, take_operation(key: 'invalid-identity')]) }.not_to change(MedicationTake, :count)
    expect(response).to have_http_status(:unprocessable_content)
    expect(source.reload.notes).to eq(original_notes)
    expect(source.medication_dose_occurrences).to be_empty
  end

  it 'validates occurrence identity even when the client UUID was already used' do
    submit([take_operation])
    expect(response).to have_http_status(:created)
    submit([take_operation(key: 'invalid-identity')])
    expect(response).to have_http_status(:unprocessable_content)
  end

  it 'preserves requests that omit occurrence identity' do
    operation = take_operation
    operation[:attributes].delete(:occurrence_key)
    expect { submit([operation]) }.to change(MedicationTake, :count).by(1)
    expect(response).to have_http_status(:created)
    expect(source.medication_dose_occurrences).to be_empty
  end

  it 'rejects a replay whose timestamp no longer matches its occurrence window' do
    operation = take_operation
    submit([operation])
    expect(response).to have_http_status(:created)
    operation[:attributes][:taken_at] = 1.day.ago.iso8601
    submit([operation])
    expect(response).to have_http_status(:unprocessable_content)
  end

  it 'requires the observed version when a queued take replaces not-taken' do
    headers
    source.reload
    record = source.medication_dose_occurrences.create!(
      window_starts_on: Date.current, position: 1, outcome: 'not_taken', resolved_at: Time.current,
      resolved_by_membership: users(:admin).person.account.first_active_household_membership
    )
    operation = take_operation
    submit([operation])
    expect(response).to have_http_status(:precondition_required)
    expect(record.reload).to be_not_taken
    operation[:attributes][:occurrence_etag] = Api::RecordEtag.for(record)
    submit([operation])
    expect(response).to have_http_status(:created)
    expect(record.reload).to be_taken
  end

  it 'checks record access again when a previously queued dose arrives' do
    headers
    operation = take_operation
    membership = users(:admin).person.account.first_active_household_membership
    PersonAccessGrant.where(household_membership: membership, person_id: source.person_id).find_each do |grant|
      grant.update!(access_level: :view)
    end
    expect { submit([operation]) }.not_to change(MedicationTake, :count)
    expect(response).to have_http_status(:forbidden)
  end
end

RSpec.describe 'API v1 queued occurrence takes' do
  context 'with a formal schedule' do
    let(:source) { schedules(:john_movicol) }

    it_behaves_like 'queued occurrence take contract'
  end

  context 'with a routine assignment' do
    let(:source) do
      create(:person_medication, :routine, person: people(:john), medication: medications(:vitamin_c),
                                           dose_cycle: :daily, max_daily_doses: 1, created_at: 2.months.ago)
    end

    it_behaves_like 'queued occurrence take contract' do
      it 'replays a monthly administration taken after the cycle start' do
        source.update!(dose_cycle: :monthly)
        operation = take_operation
        submit([operation])
        expect(response).to have_http_status(:created)
        submit([operation])
        expect(response).to have_http_status(:created)
      end
    end
  end
end
