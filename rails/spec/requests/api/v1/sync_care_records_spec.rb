require 'rails_helper'

RSpec.describe 'API v1 queued care records' do
  fixtures :all

  let(:login) { api_login(users(:admin)) }
  let(:household_id) { login.dig('household', 'id') }
  let(:headers) { api_auth_headers(login.fetch('access_token')) }

  def post_batch(*operations, request_headers: headers, **operation)
    operations << operation if operation.present?
    post api_v1_household_sync_batches_path(household_id),
         params: { batch: { operations: operations } }, headers: request_headers, as: :json
  end

  def mutation(record, resource_type, action: 'update', attributes: {})
    identifier = record.respond_to?(:portable_id) ? record.portable_id : record.id.to_s
    { resource_type: resource_type, action: action, id: identifier,
      if_match: Api::RecordEtag.for(record), attributes: attributes }
  end

  it 'creates a person and grants the same immediate management access as online creation' do
    household_id
    post_batch(resource_type: 'person', action: 'create', attributes: {
                 name: 'Offline child', date_of_birth: 8.years.ago.to_date.iso8601,
                 person_type: 'minor', has_capacity: false, account_id: accounts(:admin).id.to_s
               })
    expect(response).to have_http_status(:created)
    person = Person.find_by!(portable_id: response.parsed_body.dig('data', 'results', 0, 'record_portable_id'))
    expect(person).to have_attributes(household_id: household_id.to_i, has_capacity: false, account_id: nil)
    expect(PersonAccessGrant.active.where(person: person, access_level: :manage)).to exist
    expect(CarerRelationship.where(patient: person)).to exist
  end

  it 'updates a person with a current version and rejects stale state' do
    household_id
    operation = mutation(people(:john), 'person', attributes: { name: 'Offline name' })
    post_batch(operation.except(:if_match))
    expect(response).to have_http_status(:precondition_required)
    post_batch(operation)
    expect(response).to have_http_status(:created)
    expect(people(:john).reload.name).to eq('Offline name')
    post_batch(operation)
    expect(response).to have_http_status(:conflict)
  end

  it 'creates, updates and deletes a health event with portable references' do
    household_id
    post_batch(resource_type: 'health_event', action: 'create', attributes: {
                 person_id: people(:john).portable_id, medication_ids: [medications(:ibuprofen).portable_id],
                 title: 'Offline illness', event_kind: 'illness', severity: 'mild', started_on: Date.current.iso8601
               })
    expect(response).to have_http_status(:created)
    event = HealthEvent.find_by!(portable_id: response.parsed_body.dig('data', 'results', 0, 'record_portable_id'))
    expect(event.medications).to contain_exactly(medications(:ibuprofen))
    post_batch(mutation(event, 'health_event',
                        attributes: { severity: 'moderate', person_id: people(:jane).portable_id }))
    expect(response).to have_http_status(:created)
    expect(event.reload).to have_attributes(severity: 'moderate', person_id: people(:john).id)
    post_batch(mutation(event, 'health_event', action: 'delete'))
    expect(response).to have_http_status(:created)
    expect(HealthEvent.where(id: event.id)).not_to exist
    expect(ApiTombstone.where(record_portable_id: event.portable_id)).to exist
  end

  it 'creates, updates and deletes an unused location' do
    post_batch(resource_type: 'location', action: 'create', attributes: { name: 'Offline cupboard' })
    expect(response).to have_http_status(:created)
    location = Location.find_by!(portable_id: response.parsed_body.dig('data', 'results', 0, 'record_portable_id'))
    post_batch(mutation(location, 'location', attributes: { description: 'Upstairs' }))
    expect(response).to have_http_status(:created)
    expect(location.reload.description).to eq('Upstairs')
    post_batch(mutation(location, 'location', action: 'delete'))
    expect(response).to have_http_status(:created)
    expect(Location.where(id: location.id)).not_to exist
  end

  it 'preserves locations with administration history' do
    household_id
    create(:medication_take, schedule: schedules(:john_paracetamol), household_id: household_id)
    post_batch(mutation(locations(:home), 'location', action: 'delete'))
    expect(response).to have_http_status(:unprocessable_content)
    expect(Location.where(id: locations(:home).id)).to exist
  end

  it 'updates a review through the shared audit and evidence rules' do
    household_id
    person = people(:john)
    warfarin = person.household.medications.create!(name: 'Warfarin 1mg tablets', location: locations(:home),
                                                    dose_amount: 1, dose_unit: 'tablet')
    [warfarin, medications(:ibuprofen)].each do |medication|
      create(:person_medication, household: person.household, person: person, medication: medication)
    end
    MedicationReviewPromptSync.new(people: Person.where(id: person.id)).call
    prompt = MedicationReviewPrompt.where(person: person).sole
    evidence = prompt.evidence_text
    expect do
      post_batch(mutation(prompt, 'medication_review_prompt',
                          attributes: { status: 'not_relevant', evidence_text: 'Forged' }))
    end.to change { SecurityAuditEvent.where(event_type: 'medication_review_prompt.updated').count }.by(1)
    expect(response).to have_http_status(:created)
    expect(prompt.reload).to have_attributes(status: 'not_relevant', evidence_text: evidence)
  end

  it 'replays a review update using its server identifier without repeating the audit' do
    household_id
    person = people(:john)
    warfarin = person.household.medications.create!(name: 'Warfarin 1mg tablets', location: locations(:home),
                                                    dose_amount: 1, dose_unit: 'tablet')
    [warfarin, medications(:ibuprofen)].each do |medication|
      create(:person_medication, household: person.household, person: person, medication: medication)
    end
    MedicationReviewPromptSync.new(people: Person.where(id: person.id)).call
    prompt = MedicationReviewPrompt.where(person: person).sole
    change = mutation(prompt, 'medication_review_prompt', attributes: { status: 'not_relevant' })
    key_headers = headers.merge('Idempotency-Key' => SecureRandom.uuid)
    post_batch(change, request_headers: key_headers)
    expect(response).to have_http_status(:created)
    original = response.parsed_body
    expect { post_batch(change, request_headers: key_headers) }
      .not_to(change { SecurityAuditEvent.where(event_type: 'medication_review_prompt.updated').count })
    expect(response).to have_http_status(:created)
    expect(response.parsed_body).to eq(original)
    expect(response.headers['Idempotency-Replayed']).to eq('true')
  end

  it 'preserves shared review version errors and rolls back earlier writes' do
    household_id
    person = people(:john)
    warfarin = person.household.medications.create!(name: 'Warfarin 1mg tablets', location: locations(:home),
                                                    dose_amount: 1, dose_unit: 'tablet')
    [warfarin, medications(:ibuprofen)].each do |medication|
      create(:person_medication, household: person.household, person: person, medication: medication)
    end
    MedicationReviewPromptSync.new(people: Person.where(id: person.id)).call
    prompt = MedicationReviewPrompt.where(person: person).sole
    updater = instance_double(MedicationReviews::UpdatePrompt)
    allow(MedicationReviews::UpdatePrompt).to receive(:new).and_return(updater)
    { 'conflict' => ['sync_conflict', :conflict],
      'precondition_required' => ['precondition_required', :precondition_required] }.each do |code, (api_code, status)|
      allow(updater).to receive(:call).and_raise(MedicationReviews::UpdatePrompt::VersionError.new(code))
      post_batch({ resource_type: 'location', action: 'create', attributes: { name: 'Review rollback' } },
                 mutation(prompt, 'medication_review_prompt', attributes: { status: 'not_relevant' }))
      expect(response).to have_http_status(status)
      expect(response.parsed_body.dig('error', 'code')).to eq(api_code)
      expect(Location.where(name: 'Review rollback')).not_to exist
      expect(prompt.reload.status).to eq('needs_review')
    end
  end

  it 'rolls back earlier care records and their audit and feed effects on a later conflict' do
    household_id
    operation = mutation(people(:john), 'person', attributes: { name: 'Must roll back' }).merge(if_match: 'stale')
    counts = [Person, PersonAccessGrant, SecurityAuditEvent, ApiChangeEvent, PaperTrail::Version].map(&:count)
    post_batch({ resource_type: 'person', action: 'create', attributes: {
                 name: 'Rolled back person', date_of_birth: 30.years.ago.to_date.iso8601, person_type: 'adult'
               } }, operation)
    expect(response).to have_http_status(:conflict)
    expect([Person, PersonAccessGrant, SecurityAuditEvent, ApiChangeEvent,
            PaperTrail::Version].map(&:count)).to eq(counts)
  end

  it 'rolls back care creation when a medication update fails validation' do
    household_id
    counts = [Location, PaperTrail::Version, ApiChangeEvent].map(&:count)
    post_batch({ resource_type: 'location', action: 'create', attributes: { name: 'Discarded location' } },
               mutation(medications(:ibuprofen), 'medication', attributes: { name: nil }))
    expect(response).to have_http_status(:unprocessable_content)
    expect(medications(:ibuprofen).reload.name).to be_present
    expect([Location, PaperTrail::Version, ApiChangeEvent].map(&:count)).to eq(counts)
  end

  it 'replays the same person creation without repeating grants or audit events' do
    key_headers = headers.merge('Idempotency-Key' => SecureRandom.uuid)
    operation = { resource_type: 'person', action: 'create', attributes: {
      name: 'Replay person', date_of_birth: 30.years.ago.to_date.iso8601, person_type: 'adult'
    } }
    post_batch(operation, request_headers: key_headers)
    expect(response).to have_http_status(:created)
    original = response.parsed_body
    refreshed = api_login(users(:admin), household_id: household_id)
    key_headers = api_auth_headers(refreshed.fetch('access_token')).merge(
      'Idempotency-Key' => key_headers.fetch('Idempotency-Key')
    )
    counts = [Person, PersonAccessGrant, ApiChangeEvent].map(&:count)
    audit_count = SecurityAuditEvent.where.not(event_type: 'api.request').count
    post_batch(operation, request_headers: key_headers)
    expect(response.parsed_body).to eq(original)
    expect([Person, PersonAccessGrant, ApiChangeEvent].map(&:count)).to eq(counts)
    expect(SecurityAuditEvent.where.not(event_type: 'api.request').count).to eq(audit_count)
  end

  it 'rejects care writes after a person grant is revoked' do
    household_id
    actor = api_login(users(:jane), household_id: household_id)
    membership = ApiSession.lookup_by_access_token(actor.fetch('access_token')).household_membership
    grant = PersonAccessGrant.find_or_initialize_by(household_id: household_id, person: people(:john),
                                                    household_membership: membership, revoked_at: nil)
    grant.update!(access_level: :manage, relationship_type: :family_member)
    operation = mutation(people(:john), 'person', attributes: { name: 'Must remain private' })
    grant.update!(revoked_at: Time.current)
    post_batch(operation, request_headers: api_auth_headers(actor.fetch('access_token')))
    expect(response).to have_http_status(:not_found)
    expect(people(:john).reload.name).not_to eq('Must remain private')
  end

  it 'rejects a health event with a medication from another household' do
    household_id
    foreign = create(:medication, household: create(:household))
    post_batch(resource_type: 'health_event', action: 'create', attributes: {
                 person_id: people(:john).portable_id, medication_ids: [foreign.portable_id],
                 title: 'Invalid link', event_kind: 'illness', severity: 'mild', started_on: Date.current.iso8601
               })
    expect(response).to have_http_status(:not_found)
    expect(HealthEvent.where(title: 'Invalid link')).not_to exist
  end
end
