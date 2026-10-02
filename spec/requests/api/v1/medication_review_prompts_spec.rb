require 'rails_helper'

RSpec.describe 'API v1 medication review prompts' do
  fixtures :all

  let(:person) { people(:john) }
  let(:login) { api_login(users(:admin)) }
  let(:headers) { api_auth_headers(login.fetch('access_token')) }
  let(:path) { "/api/v1/households/#{login.dig('household', 'id')}/medication_review_prompts" }
  let(:prompt) { MedicationReviewPrompt.where(person: person).sole }

  before do
    login
    warfarin = person.household.medications.create!(name: 'Warfarin 1mg tablets', location: locations(:home),
                                                    dose_amount: 1, dose_unit: 'tablet')
    [warfarin, medications(:ibuprofen)].each do |medication|
      create(:person_medication, household: person.household, person: person, medication: medication)
    end
    MedicationReviewPromptSync.new(people: Person.where(id: person.id)).call
  end

  def review(**changes)
    attributes = { status: 'reviewed_with_practitioner', practitioner_name: 'Dr Taylor', practitioner_role: 'GP',
                   reviewed_on: Date.current.iso8601, review_note: 'Private practitioner context' }.merge(changes)
    patch "#{path}/#{prompt.id}", params: { medication_review_prompt: attributes },
                                  headers: headers.merge('If-Match' => Api::RecordEtag.for(prompt)), as: :json
  end

  it 'returns bounded evidence snapshots with a version for each prompt' do
    get path, params: { per_page: 1 }, headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(response.headers['Cache-Control']).to include('no-store')
    row = response.parsed_body.fetch('data').sole
    expect(row).to include('id' => prompt.id.to_s, 'evidence_text' => prompt.evidence_text,
                           'etag' => Api::RecordEtag.for(prompt))
    expect(response.parsed_body.fetch('meta')).to include('per_page' => 1, 'total_count' => 1)
    get "#{path}/#{prompt.id}", headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(response.headers['ETag']).to eq(Api::RecordEtag.for(prompt))
  end

  it 'applies the existing review state and priority filters' do
    get path, params: { priority: 'ask_when_convenient' }, headers: headers, as: :json
    expect(response.parsed_body.fetch('data')).to be_empty
    get path, params: { priority: 'discuss_soon' }, headers: headers, as: :json
    expect(response.parsed_body.fetch('data').size).to eq(1)
    prompt.update!(status: 'not_relevant')
    get path, headers: headers, as: :json
    expect(response.parsed_body.fetch('data')).to be_empty
    get path, params: { review_status: 'reviewed' }, headers: headers, as: :json
    expect(response.parsed_body.fetch('data').size).to eq(1)
  end

  it 'reveals hidden evidence only when requested and rejects invalid filters' do
    prompt.update!(status: 'hidden_low_signal')
    get path, params: { review_status: 'all' }, headers: headers, as: :json
    expect(response.parsed_body.fetch('data')).to be_empty
    get path, params: { show_hidden: '1' }, headers: headers, as: :json
    expect(response.parsed_body.fetch('data').size).to eq(1)
    get path, params: { review_status: 'private-invalid' }, headers: headers, as: :json
    expect(response).to have_http_status(:unprocessable_content)
    expect(response.body).not_to include('private-invalid')
  end

  it 'records practitioner attribution and an audit event without changing evidence' do
    snapshot = prompt.attributes.slice(*MedicationReviewPrompt::SNAPSHOT_ATTRIBUTES)
    expect { review(evidence_text: 'Forged evidence') }
      .to change { SecurityAuditEvent.where(event_type: 'medication_review_prompt.updated').count }.by(1)
    expect(response).to have_http_status(:ok)
    expect(prompt.reload).to have_attributes(status: 'reviewed_with_practitioner', practitioner_name: 'Dr Taylor')
    expect(prompt.reviewed_by_membership_id).to be_present
    expect(prompt.attributes.slice(*MedicationReviewPrompt::SNAPSHOT_ATTRIBUTES)).to eq(snapshot)
    event = SecurityAuditEvent.where(event_type: 'medication_review_prompt.updated').last
    expect(event.metadata).to include('previous_status' => 'needs_review', 'status' => 'reviewed_with_practitioner')
    expect(event.metadata.to_json).not_to include('Private practitioner context', 'Dr Taylor')
  end

  it 'requires practitioner context and rolls invalid changes back' do
    review(practitioner_name: '')
    expect(response).to have_http_status(:unprocessable_content)
    expect(prompt.reload.status).to eq('needs_review')
    expect(prompt.reviewed_by_membership_id).to be_nil
  end

  it 'requires a current version for review updates' do
    [nil, 'stale'].each do |etag|
      patch "#{path}/#{prompt.id}", params: { medication_review_prompt: { status: 'not_relevant' } },
                                    headers: headers.merge('If-Match' => etag), as: :json
      expect(response).to have_http_status(etag ? :conflict : :precondition_required)
      expect(prompt.reload.status).to eq('needs_review')
    end
  end

  it 'permits viewing but rejects review changes for a view-only grant' do
    actor = ApiSession.lookup_by_access_token(login.fetch('access_token')).household_membership
    PersonAccessGrant.where(household_membership: actor, person: person).find_each do |grant|
      grant.update!(access_level: :view)
    end
    get "#{path}/#{prompt.id}", headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    review
    expect(response).to have_http_status(:forbidden)
    expect(prompt.reload.status).to eq('needs_review')
  end

  it 'hides inaccessible prompts from both reads and updates' do
    actor = ApiSession.lookup_by_access_token(login.fetch('access_token')).household_membership
    PersonAccessGrant.where(household_membership: actor, person: person).find_each(&:destroy!)
    get path, headers: headers, as: :json
    expect(response.parsed_body.fetch('data')).to be_empty
    get "#{path}/#{prompt.id}", headers: headers, as: :json
    expect(response).to have_http_status(:not_found)
    review
    expect(response).to have_http_status(:not_found)
    expect(response.body).not_to include(prompt.evidence_text)
  end

  it 'rolls the review back if audit persistence fails' do
    allow(Audit::Event).to receive(:record!).and_call_original
    allow(Audit::Event).to receive(:record!).with(hash_including(event_type: 'medication_review_prompt.updated'))
                                            .and_raise(ActiveRecord::RecordInvalid.new(SecurityAuditEvent.new))
    review
    expect(response).to have_http_status(:unprocessable_content)
    expect(prompt.reload.status).to eq('needs_review')
  end

  it 'paginates multiple prompts without repeating rows' do
    other = prompt.dup
    other.person = people(:jane)
    other.save!
    get path, params: { per_page: 1 }, headers: headers, as: :json
    first_id = response.parsed_body.fetch('data').sole.fetch('id')
    expect(response.parsed_body.dig('meta', 'total_count')).to eq(2)
    get path, params: { per_page: 1, page: 2 }, headers: headers, as: :json
    second_id = response.parsed_body.fetch('data').sole.fetch('id')
    expect([first_id, second_id]).to contain_exactly(prompt.id.to_s, other.id.to_s)
  end

  it 'checks access again before replaying a cached review response' do
    headers['Idempotency-Key'] = SecureRandom.uuid
    review
    expect(response).to have_http_status(:ok)
    actor = ApiSession.lookup_by_access_token(login.fetch('access_token')).household_membership
    PersonAccessGrant.where(household_membership: actor, person: person).find_each(&:destroy!)
    review
    expect(response).to have_http_status(:not_found)
    expect(response.body).not_to include('Private practitioner context')
  end

  it 'filters private review context from request diagnostics' do
    filter = ActiveSupport::ParameterFilter.new(Rails.application.config.filter_parameters)
    expect(filter.filter('medication_review_prompt' => { 'review_note' => 'Private context' }))
      .to eq('medication_review_prompt' => '[FILTERED]')
    expect(filter.filter('attributes' => { 'review_note' => 'Private context', 'practitioner_name' => 'Dr Taylor' }))
      .to eq('attributes' => { 'review_note' => '[FILTERED]', 'practitioner_name' => '[FILTERED]' })
  end
end
