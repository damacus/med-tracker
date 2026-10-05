require 'rails_helper'

RSpec.describe 'API v1 queued assignment actions' do
  fixtures :all

  let(:login) { api_login(users(:admin)) }
  let(:household_id) { login.dig('household', 'id') }
  let(:headers) { api_auth_headers(login.fetch('access_token')) }

  def post_action(record, resource_type, action, attributes: {}, **options)
    version = options.fetch(:version) { Api::RecordEtag.for(record) }
    key = options[:key]
    operation = { resource_type: resource_type, id: record.portable_id, action: action,
                  if_match: version, attributes: attributes }
    request_headers = key ? headers.merge('Idempotency-Key' => key) : headers
    post api_v1_household_sync_batches_path(household_id), params: { batch: { operations: [operation] } },
                                                           headers: request_headers, as: :json
  end

  %w[schedule person_medication].each do |resource_type|
    it "pauses and resumes a #{resource_type} through the period workflow without replaying it" do
      household_id
      record = resource_type == 'schedule' ? schedules(:john_paracetamol) : person_medications(:jane_vitamin_d)
      record.update!(active: true)
      key = SecureRandom.uuid
      version = Api::RecordEtag.for(record)
      attributes = { reason: 'clinician_advice', note: 'Queued pause' }
      post_action(record, resource_type, 'pause', attributes: attributes, version: version, key: key)
      expect(response).to have_http_status(:created)
      original = response.parsed_body
      expect(record.reload).not_to be_active
      period = record.medication_pause_periods.sole
      expect(period).to have_attributes(reason: 'clinician_advice', note: 'Queued pause', legacy_context: false)
      post_action(record, resource_type, 'pause', attributes: attributes, version: version, key: key)
      expect(response.parsed_body).to eq(original)
      expect(record.medication_pause_periods.count).to eq(1)
      post_action(record, resource_type, 'resume')
      expect(response).to have_http_status(:created)
      expect(record.reload).to be_active
      expect(period.reload.ended_at).to be_present
    end

    it "requires a current version and rejects retired #{resource_type} actions" do
      household_id
      record = resource_type == 'schedule' ? schedules(:john_paracetamol) : person_medications(:jane_vitamin_d)
      post_action(record, resource_type, 'pause', version: nil)
      expect(response).to have_http_status(:precondition_required)
      post_action(record, resource_type, 'pause', version: 'stale')
      expect(response).to have_http_status(:conflict)
      record.retire!
      post_action(record, resource_type, 'resume')
      expect(response).to have_http_status(:not_found)
    end
  end

  it 'reorders direct assignments once and rejects invalid directions' do
    household_id
    person = people(:john)
    position = person.person_medications.maximum(:position).to_i + 1
    first = create(:person_medication, person: person, household: person.household, position: position)
    second = create(:person_medication, person: person, household: person.household, position: position + 1)
    key = SecureRandom.uuid
    version = Api::RecordEtag.for(second)
    post_action(second, 'person_medication', 'reorder', attributes: { direction: 'up' }, version: version, key: key)
    expect(response).to have_http_status(:created)
    expect([first.reload.position, second.reload.position]).to eq([position + 1, position])
    post_action(second, 'person_medication', 'reorder', attributes: { direction: 'up' }, version: version, key: key)
    expect(response).to have_http_status(:created)
    expect([first.reload.position, second.reload.position]).to eq([position + 1, position])
    post_action(second, 'person_medication', 'reorder', attributes: { direction: 'sideways' })
    expect(response).to have_http_status(:unprocessable_content)
  end
end
