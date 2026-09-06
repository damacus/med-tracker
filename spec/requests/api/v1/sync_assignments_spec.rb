require 'rails_helper'

RSpec.describe 'API v1 queued schedules and assignments' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules,
           :person_medications, :medication_takes, :carer_relationships

  let(:login_data) { api_login(users(:admin)) }
  let(:household_id) { login_data.dig('household', 'id') }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')) }

  %w[schedule person_medication].each do |resource_type|
    context "with #{resource_type}" do
      let(:record) { assignment_record(resource_type) }

      it 'creates a record using permitted portable references' do
        attributes = creation_attributes(resource_type)
        post_batch({ action: 'create', resource_type: resource_type, attributes: attributes })

        expect(response).to have_http_status(:created)
        result = response.parsed_body.dig('data', 'results', 0)
        created = assignment_class(resource_type).find_by!(portable_id: result.fetch('record_portable_id'))
        expect(created).to have_attributes(person: people(:john), medication: medications(:ibuprofen))
        expect(result).to include('action' => 'create', 'record_type' => created.class.name)
        expect(ApiChangeEvent.where(record_portable_id: created.portable_id, action: 'create')).to exist
      end

      it 'updates a record with its latest version' do
        post_batch(mutation(record, resource_type, attributes: { notes: 'Updated offline' }))

        expect(response).to have_http_status(:created)
        expect(record.reload.notes).to eq('Updated offline')
        expect(response.parsed_body.dig('data', 'results', 0, 'etag')).to eq(Api::RecordEtag.for(record))
      end

      it 'retires the source without deleting its past doses and tells sync clients it was removed' do
        take = create(:medication_take, resource_type.to_sym => record, household_id: household_id)
        original_take = take.attributes
        original_id = record.portable_id

        post_batch(mutation(record, resource_type, action: 'delete'))

        expect(response).to have_http_status(:created)
        expect(record.reload).to have_attributes(active: false, portable_id: original_id)
        expect(record.retired_at).to be_present
        expect(take.reload.attributes).to eq(original_take)
        expect(ApiTombstone.where(record_portable_id: original_id, action: 'delete').count).to eq(1)
        expect(PaperTrail::Version.where(item_type: record.class.name, item_id: record.id, event: 'update')).to exist
      end

      it 'requires a version and rejects an older version without changing the record' do
        operation = mutation(record, resource_type)
        post_batch(operation.except(:if_match))
        expect(response).to have_http_status(:precondition_required)
        expect(response.parsed_body.dig('error', 'code')).to eq('precondition_required')

        record.update!(notes: 'Changed elsewhere', updated_at: 1.minute.from_now)
        post_batch(operation)
        expect(response).to have_http_status(:conflict)
        expect(response.parsed_body.dig('error', 'code')).to eq('sync_conflict')
        expect(record.reload.notes).to eq('Changed elsewhere')
      end

      it 'replays a creation and rejects reuse of the key for a different batch' do
        operation = { action: 'create', resource_type: resource_type, attributes: creation_attributes(resource_type) }
        retry_headers = headers.merge('Idempotency-Key' => SecureRandom.uuid)
        post_batch(operation, request_headers: retry_headers)
        expect(response).to have_http_status(:created)
        original_response = response.parsed_body
        original_counts = mutation_counts

        post_batch(operation, request_headers: retry_headers)
        expect(response.parsed_body).to eq(original_response)
        expect(mutation_counts).to eq(original_counts)

        operation[:attributes][:notes] = 'Different request'
        post_batch(operation, request_headers: retry_headers)
        expect(response).to have_http_status(:conflict)
        expect(response.parsed_body.dig('error', 'code')).to eq('idempotency_key_reused')
        expect(mutation_counts).to eq(original_counts)
      end

      it 'replays removal without repeating audit or deletion markers' do
        operation = mutation(record, resource_type, action: 'delete')
        retry_headers = headers.merge('Idempotency-Key' => SecureRandom.uuid)
        post_batch(operation, request_headers: retry_headers)
        expect(response).to have_http_status(:created)
        original_counts = mutation_counts
        post_batch(operation, request_headers: retry_headers)
        expect(response).to have_http_status(:created)
        expect(mutation_counts).to eq(original_counts)
      end

      it 'rejects malformed fields and dosage options belonging to another medicine' do
        attributes = creation_attributes(resource_type)
        attributes[:dose_amount] = '-1'
        post_batch({ action: 'create', resource_type: resource_type, attributes: attributes })
        expect(response).to have_http_status(:unprocessable_content)
        expect(response.parsed_body.dig('error', 'message')).to eq('operation 0 attributes are invalid')

        attributes[:dose_amount] = '200'
        attributes[:source_dosage_option_id] = medications(:paracetamol).dosage_records.first!.portable_id
        post_batch({ action: 'create', resource_type: resource_type, attributes: attributes })
        expect(response).to have_http_status(:unprocessable_content)
      end

      it 'accepts string database identifiers but rejects numeric JSON references' do
        attributes = creation_attributes(resource_type).merge(
          person_id: people(:john).id, medication_id: medications(:ibuprofen).id.to_s
        )
        post_batch({ action: 'create', resource_type: resource_type, attributes: attributes })
        expect(response).to have_http_status(:unprocessable_content)
        expect(response.parsed_body.dig('error', 'errors', 'person_id')).to eq(['must be a string'])

        attributes[:person_id] = attributes[:person_id].to_s
        post_batch({ action: 'create', resource_type: resource_type, attributes: attributes })
        expect(response).to have_http_status(:created)
      end

      it 'cannot reassign a person or write server-owned fields' do
        original = record.attributes.slice('person_id', 'household_id', 'portable_id', 'active', 'retired_at')
        fields = { person_id: people(:bob).portable_id, household_id: '0', portable_id: SecureRandom.uuid,
                   active: false, retired_at: Time.current.iso8601, notes: 'Permitted' }
        post_batch(mutation(record, resource_type, attributes: fields))
        expect(response).to have_http_status(:created)
        expect(record.reload.attributes.slice(*original.keys)).to eq(original)
        expect(record.notes).to eq('Permitted')
      end

      it 'rejects updates to retired records and unsupported actions' do
        operation = mutation(record, resource_type)
        post_batch(operation.merge(action: 'pause'))
        expect(response).to have_http_status(:unprocessable_content)
        expect(response.parsed_body.dig('error', 'code')).to eq('sync_operation_unsupported')

        record.retire!
        post_batch(operation)
        expect(response).to have_http_status(:not_found)
      end

      it 'rejects invalid enum values without echoing the submitted value' do
        field = resource_type == 'schedule' ? :schedule_type : :administration_kind
        operation = mutation(record, resource_type, attributes: { field => 'private-invalid-input' })
        post_batch(operation)
        expect(response).to have_http_status(:unprocessable_content)
        expect(response.body).not_to include('private-invalid-input')
      end

      it 'hides records and related references from another household' do
        household_id
        other = create(:household)
        person = create(:person, household: other)
        medication = create(:medication, household: other, location: create(:location, household: other))
        foreign = create(resource_type.to_sym, household: other, person: person, medication: medication)
        original = foreign.attributes
        post_batch(mutation(foreign, resource_type))
        expect(response).to have_http_status(:not_found)
        expect(foreign.reload.attributes).to eq(original)

        { person_id: person.portable_id, medication_id: medication.portable_id }.each do |field, value|
          attributes = creation_attributes(resource_type).merge(field => value)
          post_batch({ action: 'create', resource_type: resource_type, attributes: attributes })
          expect(response).to have_http_status(:not_found)
          expect(response.body).not_to include(person.name, medication.name, value)
        end
      end

      it 'requires manage access for creation and removal, not just visibility' do
        attributes = creation_attributes(resource_type)
        actor_session = actor_access(:doctor, people(:john), :view)
        post_batch({ action: 'create', resource_type: resource_type, attributes: attributes },
                   request_headers: actor_session.fetch(:headers))
        expect(response).to have_http_status(:forbidden)

        actor_session = actor_access(:doctor, record.person, :view)
        post_batch(mutation(record, resource_type, action: 'delete'),
                   request_headers: actor_session.fetch(:headers))
        expect(response).to have_http_status(:forbidden)
        expect(record.reload.retired_at).to be_nil
      end

      it 'allows a member to manage their own assignment with a self grant' do
        person = users(:jane).person
        own_record = create(resource_type.to_sym, household_id: household_id, person: person,
                                                  medication: medications(:ibuprofen))
        actor_session = actor_access(:jane, person, :manage)
        actor_session.fetch(:grant).update!(relationship_type: :self)
        post_batch(mutation(own_record, resource_type), request_headers: actor_session.fetch(:headers))
        expect(response).to have_http_status(:created)
      end

      it 'rolls back creation, updates, retirement and their change history after a late failure' do
        other_record = create(resource_type.to_sym, household_id: household_id,
                                                    person: people(:bob), medication: medications(:paracetamol))
        before_record = record.attributes
        before_other = other_record.attributes
        operations = [
          { action: 'create', resource_type: resource_type, attributes: creation_attributes(resource_type) },
          mutation(record, resource_type),
          mutation(other_record, resource_type, action: 'delete'),
          { action: 'update', resource_type: 'health_event', id: SecureRandom.uuid, if_match: 'missing' }
        ]
        original_counts = mutation_counts
        post_batch(*operations)

        expect(response).to have_http_status(:not_found)
        expect(record.reload.attributes).to eq(before_record)
        expect(other_record.reload.attributes).to eq(before_other)
        expect(mutation_counts).to eq(original_counts)
      end

      it 'rolls back retirement when a required deletion marker cannot be saved' do
        operation = mutation(record, resource_type, action: 'delete')
        original = record.attributes
        original_counts = mutation_counts
        allow(ApiTombstone).to receive(:create!).and_raise(ActiveRecord::RecordInvalid.new(ApiTombstone.new))

        post_batch(operation)

        expect(response).to have_http_status(:unprocessable_content)
        expect(record.reload.attributes).to eq(original)
        expect(mutation_counts).to eq(original_counts)
      end

      it 'preserves dated pause history when retiring a source' do
        session = ApiSession.lookup_by_access_token(login_data.fetch('access_token'))
        pause = record.medication_pause_periods.create!(
          started_at: 1.day.ago, reason: 'out_of_supply', recorded_by_membership: session.household_membership
        )
        original = pause.attributes
        post_batch(mutation(record, resource_type, action: 'delete'))

        expect(response).to have_http_status(:created)
        expect(pause.reload.attributes).to eq(original)
      end

      %i[doctor carer parent].each do |actor|
        it "requires manage access for #{actor} to change an assignment" do
          operation = mutation(record, resource_type)
          actor_session = actor_access(actor, record.person, :record)
          post_batch(operation, request_headers: actor_session.fetch(:headers))
          expect(response).to have_http_status(:forbidden)

          actor_session.fetch(:grant).update!(access_level: :manage)
          post_batch(operation, request_headers: actor_session.fetch(:headers))
          expect(response).to have_http_status(:created)
        end
      end

      it 'rejects revoked and expired grants without disclosing record content' do
        operation = mutation(record, resource_type)
        actor_session = actor_access(:nurse, record.person, :manage)
        actor_session.fetch(:grant).update!(expires_at: 1.minute.ago)
        post_batch(operation, request_headers: actor_session.fetch(:headers))
        expect(response).to have_http_status(:not_found)
        expect(response.body).not_to include(record.person.name, record.medication.name)

        actor_session.fetch(:grant).update!(expires_at: nil, revoked_at: Time.current)
        post_batch(operation, request_headers: actor_session.fetch(:headers))
        expect(response).to have_http_status(:not_found)
      end
    end
  end

  it 'rolls back an already recorded take and its stock change when a later assignment is invalid' do
    household_id
    medication = create(:medication, household_id: household_id, location: locations(:home), current_supply: 20)
    source = create(:schedule, household_id: household_id, person: people(:john),
                               medication: medication, start_date: Date.current,
                               end_date: 1.month.from_now)
    operation = { action: 'create', resource_type: 'medication_take', attributes: {
      source_type: 'schedule', source_id: source.portable_id, client_uuid: SecureRandom.uuid,
      taken_at: Time.current.iso8601
    } }
    bad_assignment = { action: 'create', resource_type: 'person_medication',
                       attributes: creation_attributes('person_medication').merge(dose_amount: '-1') }
    original_supply = source.medication.current_supply
    original_counts = mutation_counts

    post_batch(operation, bad_assignment)

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.parsed_body.dig('error', 'message')).to eq('operation 1 attributes are invalid')
    expect(source.medication.reload.current_supply).to eq(original_supply)
    expect(mutation_counts).to eq(original_counts)
  end

  def actor_access(actor, person, access_level)
    data = api_login(users(actor), household_id: household_id)
    session = ApiSession.lookup_by_access_token(data.fetch('access_token'))
    grant = PersonAccessGrant.find_or_initialize_by(household_id: household_id, person: person,
                                                    household_membership: session.household_membership, revoked_at: nil)
    grant.update!(access_level: access_level, relationship_type: :family_member)
    { headers: api_auth_headers(data.fetch('access_token')), grant: grant }
  end

  def mutation_counts
    [Schedule, PersonMedication, MedicationTake, PaperTrail::Version, ApiChangeEvent, ApiTombstone].map(&:count)
  end

  def assignment_class(resource_type)
    resource_type == 'schedule' ? Schedule : PersonMedication
  end

  def assignment_record(resource_type)
    household_id
    resource_type == 'schedule' ? schedules(:john_paracetamol) : person_medications(:jane_vitamin_d)
  end

  def creation_attributes(resource_type)
    household_id
    attributes = {
      person_id: people(:john).portable_id,
      medication_id: medications(:ibuprofen).portable_id,
      dose_amount: '200', dose_unit: 'mg', notes: 'Queued assignment'
    }
    return attributes unless resource_type == 'schedule'

    attributes.merge(start_date: Date.current.iso8601, end_date: 1.month.from_now.to_date.iso8601,
                     schedule_type: 'weekly', schedule_config: { weekdays: ['monday'] })
  end

  def mutation(record, resource_type, action: 'update', attributes: { notes: 'Queued update' })
    { action: action, resource_type: resource_type, id: record.portable_id,
      if_match: Api::RecordEtag.for(record), attributes: attributes }
  end

  def post_batch(*operations, request_headers: headers)
    post api_v1_household_sync_batches_path(household_id),
         params: { batch: { operations: operations } }, headers: request_headers, as: :json
  end
end
