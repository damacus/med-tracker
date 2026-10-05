require 'rails_helper'

RSpec.describe 'API v1 protected reports' do
  fixtures :all

  let(:login) { api_login(users(:admin)) }
  let(:headers) { api_auth_headers(login.fetch('access_token')) }
  let(:person) { people(:john) }
  let(:path) { "/api/v1/households/#{login.dig('household', 'id')}/reports/health_history" }
  let(:filters) { { person_id: person.portable_id, start_date: '2026-09-01', end_date: '2026-09-09' } }

  before do
    travel_to Time.zone.local(2026, 9, 9, 12)
    login
    HealthEvent.create!(person: person, event_kind: :illness, title: 'Report illness', started_on: Date.current)
    HealthEvent.create!(person: people(:jane), event_kind: :illness, title: 'Other person private illness',
                        started_on: Date.current)
  end

  it 'returns a typed person-specific chronology and audits the JSON download' do
    expect { get path, params: filters, headers: headers, as: :json }
      .to change { SecurityAuditEvent.where(event_type: 'health_history_report.downloaded').count }.by(1)
    expect(response).to have_http_status(:ok)
    expect(response.headers['Cache-Control']).to include('no-store')
    data = response.parsed_body.fetch('data')
    expect(data.dig('person', 'id')).to eq(person.id.to_s)
    expect(data.fetch('chronology').pluck('title')).to include('Report illness')
    expect(response.body).not_to include('Other person private illness')
    expect(data.fetch('medication_takes')).to be_empty
  end

  it 'returns the existing PDF report and records its download' do
    get "#{path}.pdf", params: filters, headers: headers
    expect(response).to have_http_status(:ok)
    expect(response.media_type).to eq('application/pdf')
    expect(response.headers['Cache-Control']).to include('no-store')
    expect(response.headers['Content-Disposition']).to include('attachment', '2026-09-01-to-2026-09-09.pdf')
    expect(pdf_text(response.body)).to include('Report illness', person.name)
    expect(pdf_text(response.body)).not_to include('Other person private illness')
    expect(SecurityAuditEvent.where(event_type: 'health_history_report.downloaded').last.metadata)
      .to include('format' => 'pdf', 'person_id' => person.id)
  end

  it 'includes actual doses only when requested and identifies routine administrations correctly' do
    source = create(:person_medication, :routine, person: person, medication: medications(:vitamin_c))
    create(:medication_take, :for_person_medication, person_medication: source, taken_at: Time.current)
    get path, params: filters.merge(include_medication_takes: '1'), headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    rows = response.parsed_body.dig('data', 'medication_takes')
    expect(rows).to include(include('source_type' => 'routine', 'medication_name' => source.medication.display_name))
    get "#{path}.pdf", params: filters.merge(include_medication_takes: '1'), headers: headers
    expect(response).to have_http_status(:ok)
    expect(pdf_text(response.body)).to include('Routine')
  end

  it 'rejects invalid, reversed and excessive date ranges without a download event' do
    invalid = [{ start_date: 'private-invalid' }, { start_date: '9 September 2026' },
               { start_date: '2026-09-10' }, { start_date: '2020-01-01' }]
    invalid.each do |changes|
      expect { get path, params: filters.merge(changes), headers: headers, as: :json }
        .not_to(change { SecurityAuditEvent.where(event_type: 'health_history_report.downloaded').count })
      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).not_to include('private-invalid', 'Report illness')
    end
  end

  it 'requires manage access to download health history' do
    actor = ApiSession.lookup_by_access_token(login.fetch('access_token')).household_membership
    PersonAccessGrant.where(household_membership: actor, person: person).find_each do |grant|
      grant.update!(access_level: :view)
    end
    get path, params: filters, headers: headers, as: :json
    expect(response).to have_http_status(:not_found)
    expect(response.body).not_to include('Report illness')
  end

  it 'hides foreign people and requires a selected person' do
    foreign = create(:person, household: create(:household))
    get path, params: filters.merge(person_id: foreign.portable_id), headers: headers, as: :json
    expect(response).to have_http_status(:not_found)
    get path, params: filters.except(:person_id), headers: headers, as: :json
    expect(response).to have_http_status(:bad_request)
  end

  it 'returns a stable JSON error when PDF rendering fails' do
    allow(Observability::EmergencyDiagnostic).to receive(:write).and_call_original
    allow(Observability::CanonicalLogger).to receive(:write).and_call_original
    renderer = instance_double(Reports::HealthHistoryPdf)
    allow(Reports::HealthHistoryPdf).to receive(:new).and_return(renderer)
    allow(renderer).to receive(:render).and_raise(Reports::PdfRenderer::Error, 'Private renderer clinical details')
    get "#{path}.pdf", params: filters, headers: headers
    expect(response).to have_http_status(:service_unavailable)
    expect(response.media_type).to eq('application/json')
    expect(response.parsed_body.dig('error', 'code')).to eq('report_unavailable')
    expect(response.body).not_to include('Private renderer clinical details', 'Report illness')
    expect(Observability::EmergencyDiagnostic).not_to have_received(:write)
    expect(Observability::CanonicalLogger).to have_received(:write).with(satisfy do |event|
      event.to_h['medtracker.diagnostic.component'] == 'api_report_export'
    end)
    expect(SecurityAuditEvent.where(event_type: 'health_history_report.downloaded')).to be_empty
  end
end
