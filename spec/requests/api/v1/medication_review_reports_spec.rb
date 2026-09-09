require 'rails_helper'

RSpec.describe 'API v1 medication review reports' do
  fixtures :all

  let(:login) { api_login(users(:admin)) }
  let(:headers) { api_auth_headers(login.fetch('access_token')) }
  let(:person) { people(:john) }
  let(:path) { "/api/v1/households/#{login.dig('household', 'id')}/reports/medication_reviews" }
  let(:prompt) { MedicationReviewPrompt.where(person: person).sole }

  before do
    login
    medicine = person.household.medications.create!(name: 'Warfarin 1mg tablets', location: locations(:home),
                                                    dose_amount: 1, dose_unit: 'tablet')
    [medicine, medications(:ibuprofen)].each do |medication|
      create(:person_medication, person: person, medication: medication)
    end
    MedicationReviewPromptSync.new(people: Person.where(id: person.id)).call
  end

  it 'returns the selected person review snapshot and audits the JSON download' do
    expect { get path, params: { person_id: person.portable_id }, headers: headers, as: :json }
      .to change { SecurityAuditEvent.where(event_type: 'medication_review_report.downloaded').count }.by(1)
    expect(response).to have_http_status(:ok)
    expect(response.headers['Cache-Control']).to include('no-store')
    data = response.parsed_body.fetch('data')
    expect(data.dig('person', 'id')).to eq(person.id.to_s)
    expect(data.fetch('prompts').sole).to include('id' => prompt.id.to_s, 'evidence_text' => prompt.evidence_text)
  end

  it 'renders the existing protected PDF with the same evidence snapshot' do
    get "#{path}.pdf", params: { person_id: person.id }, headers: headers
    expect(response).to have_http_status(:ok)
    expect(response.media_type).to eq('application/pdf')
    expect(response.headers['Cache-Control']).to include('no-store')
    expect(pdf_text(response.body)).to include(person.name, 'Warfarin 1mg tablets', 'Ibuprofen')
    expect(SecurityAuditEvent.where(event_type: 'medication_review_report.downloaded').last.metadata)
      .to include('format' => 'pdf')
  end

  it 'uses the web report visibility and explicit status filter' do
    prompt.update!(status: 'hidden_low_signal')
    get path, params: { person_id: person.id }, headers: headers, as: :json
    expect(response.parsed_body.dig('data', 'prompts')).to be_empty
    get path, params: { person_id: person.id, status: 'hidden_low_signal' }, headers: headers, as: :json
    expect(response.parsed_body.dig('data', 'prompts').size).to eq(1)
  end

  it 'permits a viewer to export only their accessible review snapshot' do
    actor = ApiSession.lookup_by_access_token(login.fetch('access_token')).household_membership
    grants = PersonAccessGrant.where(household_membership: actor, person: person)
    grants.find_each { |grant| grant.update!(access_level: :view) }
    get path, params: { person_id: person.id }, headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    grants.find_each(&:destroy!)
    get path, params: { person_id: person.id }, headers: headers, as: :json
    expect(response).to have_http_status(:not_found)
    expect(response.body).not_to include(prompt.evidence_text)
  end

  it 'rejects invalid status, unsupported date filters and missing person selection' do
    [{ status: 'private-invalid' }, { start_date: '2026-09-01' }, { format: 'xml' }].each do |extra|
      get path, params: { person_id: person.id }.merge(extra), headers: headers
      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).not_to include('private-invalid', prompt.evidence_text)
    end
    get path, headers: headers, as: :json
    expect(response).to have_http_status(:bad_request)
  end

  it 'returns a stable error without a download audit when PDF rendering fails' do
    renderer = instance_double(Reports::MedicationReviewPdf)
    allow(Reports::MedicationReviewPdf).to receive(:new).and_return(renderer)
    allow(renderer).to receive(:render).and_raise(Reports::PdfRenderer::Error, 'Private clinical renderer data')
    get "#{path}.pdf", params: { person_id: person.id }, headers: headers
    expect(response).to have_http_status(:service_unavailable)
    expect(response.parsed_body.dig('error', 'code')).to eq('report_unavailable')
    expect(response.body).not_to include('Private clinical renderer data')
    expect(SecurityAuditEvent.where(event_type: 'medication_review_report.downloaded')).to be_empty
  end
end
