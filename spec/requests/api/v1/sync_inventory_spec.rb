require 'rails_helper'

RSpec.describe 'API v1 queued inventory' do
  fixtures :all

  let(:login) { api_login(users(:admin)) }
  let(:household_id) { login.dig('household', 'id') }
  let(:headers) { api_auth_headers(login.fetch('access_token')) }
  let(:medication) do
    household_id
    medications(:ibuprofen)
  end

  def post_batch(*operations, request_headers: headers)
    post api_v1_household_sync_batches_path(household_id),
         params: { batch: { operations: operations } }, headers: request_headers, as: :json
  end

  def mutation(action, attributes = {}, record: medication, resource_type: 'medication')
    { resource_type: resource_type, action: action, id: record.portable_id,
      if_match: Api::RecordEtag.for(record), attributes: attributes }
  end

  it 'creates medication with a portable location and the same permitted fields as online' do
    household_id
    post_batch({ resource_type: 'medication', action: 'create', attributes: {
                 name: 'Offline stock', location_id: locations(:home).portable_id,
                 dose_amount: '5', dose_unit: 'ml', current_supply: '20', reorder_threshold: '5',
                 household_id: '-1'
               } })
    expect(response).to have_http_status(:created)
    record = Medication.find_by!(portable_id: response.parsed_body.dig('data', 'results', 0, 'record_portable_id'))
    expect(record).to have_attributes(household_id: household_id.to_i, location: locations(:home), current_supply: 20)
    expect(record.created_by_membership_id).to be_present
  end

  it 'updates medication fields without accepting ownership fields' do
    post_batch(mutation('update', { description: 'Offline description', household_id: '-1' }))
    expect(response).to have_http_status(:created)
    expect(medication.reload).to have_attributes(description: 'Offline description', household_id: household_id.to_i)
  end

  it 'creates and updates a dosage option without moving it to another medication' do
    post_batch({ resource_type: 'medication_dosage_option', action: 'create', attributes: {
                 medication_id: medication.portable_id, amount: '2.5', unit: 'ml', frequency: 'daily',
                 current_supply: '25',
                 default_max_daily_doses: 1, default_min_hours_between_doses: '0', default_dose_cycle: 'daily'
               } })
    expect(response).to have_http_status(:created)
    dosage = MedicationDosageOption.find_by!(portable_id: response.parsed_body.dig('data', 'results', 0,
                                                                                   'record_portable_id'))
    attributes = { description: 'Updated option', medication_id: medications(:paracetamol).portable_id }
    post_batch(mutation('update', attributes,
                        record: dosage, resource_type: 'medication_dosage_option'))
    expect(response).to have_http_status(:created)
    expect(dosage.reload).to have_attributes(description: 'Updated option', medication_id: medication.id)
  end

  it 'adjusts inventory with an ETag and preserves the shared audit event' do
    operation = mutation('adjust_inventory', { new_quantity: '12.5', reason: 'Counted offline' })
    post_batch(operation.except(:if_match))
    expect(response).to have_http_status(:precondition_required)
    post_batch(operation)
    expect(response).to have_http_status(:created)
    expect(medication.reload.current_supply).to eq(12.5)
    expect(medication.versions.last.event).to include('adjust inventory')
    post_batch(operation)
    expect(response).to have_http_status(:conflict)
  end

  it 'records order and receipt through the existing workflow' do
    post_batch(mutation('mark_as_ordered',
                        { supplier: 'Pharmacy', quantity: '30', expected_arrival_on: Date.tomorrow.iso8601 }))
    expect(response).to have_http_status(:created)
    expect(medication.reload).to have_attributes(reorder_status: 'ordered', order_supplier: 'Pharmacy',
                                                 order_quantity: 30)
    post_batch(mutation('mark_as_received'))
    expect(response).to have_http_status(:created)
    expect(medication.reload.reorder_status).to eq('received')
  end

  it 'removes stock once on a lost-response retry without adding a clinical dose' do
    medication.update!(current_supply: 20)
    operation = mutation('remove_stock', { quantity: '2.5', reason: 'dropped', submission_id: SecureRandom.uuid })
    retry_headers = headers.merge('Idempotency-Key' => SecureRandom.uuid)
    take_count = MedicationTake.count
    post_batch(operation, request_headers: retry_headers)
    expect(response).to have_http_status(:created)
    original = response.parsed_body
    post_batch(operation, request_headers: retry_headers)
    expect(response.parsed_body).to eq(original)
    post_batch(operation)
    expect(response).to have_http_status(:created)
    expect(medication.reload.current_supply).to eq(17.5)
    expect(RemoveMedicationStockService.history(medication).count).to eq(1)
    expect(MedicationTake.count).to eq(take_count)
  end

  it 'rejects invalid stock quantities and rolls back earlier inventory and its audit' do
    medication.update!(current_supply: 20)
    counts = [PaperTrail::Version, ApiChangeEvent].map(&:count)
    post_batch(mutation('adjust_inventory', { new_quantity: '15' }),
               mutation('remove_stock', { quantity: '1e1', reason: 'dropped', submission_id: SecureRandom.uuid })
                 .except(:if_match))
    expect(response).to have_http_status(:unprocessable_content)
    expect(medication.reload.current_supply).to eq(20)
    expect([PaperTrail::Version, ApiChangeEvent].map(&:count)).to eq(counts)
  end

  it 'preserves the online distinction between ordering visible medicine and changing its stock' do
    household_id
    actor = api_login(users(:jane), household_id: household_id)
    membership = ApiSession.lookup_by_access_token(actor.fetch('access_token')).household_membership
    grant = PersonAccessGrant.find_or_initialize_by(household_id: household_id, person: people(:john),
                                                    household_membership: membership, revoked_at: nil)
    grant.update!(access_level: :view, relationship_type: :family_member)
    medicine = schedules(:john_paracetamol).medication
    actor_headers = api_auth_headers(actor.fetch('access_token'))
    post_batch(mutation('mark_as_ordered', { supplier: 'Pharmacy' }, record: medicine), request_headers: actor_headers)
    expect(response).to have_http_status(:created)
    medicine.reload
    supply = medicine.current_supply
    post_batch(mutation('adjust_inventory', { new_quantity: '100' }, record: medicine), request_headers: actor_headers)
    expect(response).to have_http_status(:forbidden)
    expect(medicine.reload.current_supply).to eq(supply)
  end
end
