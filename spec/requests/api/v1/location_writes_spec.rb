require 'rails_helper'

RSpec.describe 'API v1 location writes' do
  fixtures :accounts, :people, :users, :locations, :households

  let(:login) { api_login(users(:admin)) }
  let(:headers) { api_auth_headers(login.fetch('access_token')) }
  let(:path) { "/api/v1/households/#{login.dig('household', 'id')}/locations" }
  let(:location) { Location.create!(household: users(:admin).person.household, name: 'API storage') }

  before { login }

  def version_headers
    headers.merge('If-Match' => Api::RecordEtag.for(location))
  end

  it 'creates household-owned storage with a portable identity and immediate read access' do
    post path, params: { location: { name: 'Travel bag', description: 'Daily supplies', household_id: 0 } },
               headers: headers, as: :json
    expect(response).to have_http_status(:created)
    record = Location.find(response.parsed_body.dig('data', 'id'))
    expect(record.household_id).to eq(users(:admin).person.household_id)
    expect(record.portable_id).to be_present
    expect(response.headers['ETag']).to eq(Api::RecordEtag.for(record))
    get "#{path}/#{record.portable_id}", headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'name')).to eq('Travel bag')
  end

  it 'rejects blank and duplicate names without a location or domain audit' do
    location
    ['', 'api storage'].each do |name|
      expect do
        post path, params: { location: { name: name } }, headers: headers, as: :json
      end.not_to change(Location, :count)
      expect(response).to have_http_status(:unprocessable_content)
    end
  end

  it 'updates by portable identity with the observed version' do
    patch "#{path}/#{location.portable_id}", params: { location: { name: 'Updated storage' } },
                                             headers: version_headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(location.reload.name).to eq('Updated storage')
    expect(response.headers['ETag']).to eq(Api::RecordEtag.for(location))
  end

  it 'requires a current version for edits and deletes' do
    ['stale', nil].each do |etag|
      current_headers = headers.merge('If-Match' => etag)
      patch "#{path}/#{location.id}", params: { location: { name: 'Stale edit' } }, headers: current_headers, as: :json
      expect(response).to have_http_status(etag ? :conflict : :precondition_required)
      delete "#{path}/#{location.id}", headers: current_headers, as: :json
      expect(response).to have_http_status(etag ? :conflict : :precondition_required)
      expect(location.reload.name).to eq('API storage')
    end
  end

  it 'preserves the newer version when an edit races another change' do
    observed = version_headers
    location.update!(description: 'Newer facts')
    patch "#{path}/#{location.id}", params: { location: { name: 'Stale edit' } }, headers: observed, as: :json
    expect(response).to have_http_status(:conflict)
    expect(location.reload).to have_attributes(name: 'API storage', description: 'Newer facts')
  end

  it 'rejects invalid updates without changing the description' do
    patch "#{path}/#{location.id}", params: { location: { name: '', description: 'Partial edit' } },
                                    headers: version_headers, as: :json
    expect(response).to have_http_status(:unprocessable_content)
    expect(location.reload.description).to be_nil
  end

  it 'deletes unreferenced storage with a current version' do
    location
    expect { delete "#{path}/#{location.id}", headers: version_headers, as: :json }.to change(Location, :count).by(-1)
    expect(response).to have_http_status(:no_content)
  end

  it 'retains a location and its medication administration history' do
    medication = create(:medication, household: location.household, location: location)
    schedule = create(:schedule, household: location.household, medication: medication)
    take = create(:medication_take, :for_schedule, household: location.household, schedule: schedule)
    delete "#{path}/#{location.id}", headers: version_headers, as: :json
    expect(response).to have_http_status(:unprocessable_content)
    expect(location.reload).to be_persisted
    expect(take.reload).to be_persisted
    expect(medication.reload).to be_persisted
  end

  it 'does not disclose or edit foreign household storage' do
    foreign = Location.create!(household: create(:household), name: 'Private storage')
    foreign_headers = headers.merge('If-Match' => Api::RecordEtag.for(foreign))
    patch "#{path}/#{foreign.portable_id}", params: { location: { name: 'Changed' } },
                                            headers: foreign_headers, as: :json
    expect(response).to have_http_status(:not_found)
    expect(response.body).not_to include('Private storage')
    expect(foreign.reload.name).to eq('Private storage')
  end

  it 'requires inventory management permission for every write' do
    member = api_login(users(:jane))
    member_headers = api_auth_headers(member.fetch('access_token')).merge('If-Match' => Api::RecordEtag.for(location))
    post path, params: { location: { name: 'Denied storage' } }, headers: member_headers, as: :json
    expect(response).to have_http_status(:forbidden)
    patch "#{path}/#{location.id}", params: { location: { name: 'Denied storage' } },
                                    headers: member_headers, as: :json
    expect(response).to have_http_status(:forbidden)
    delete "#{path}/#{location.id}", headers: member_headers, as: :json
    expect(response).to have_http_status(:forbidden)
  end

  it 'retains a location with a saved not-taken decision' do
    medication = create(:medication, household: location.household, location: location)
    source = create(:schedule, household: location.household, medication: medication)
    actor = ApiSession.lookup_by_access_token(login.fetch('access_token')).household_membership
    outcome = source.medication_dose_occurrences.create!(window_starts_on: Date.current, position: 1,
                                                         outcome: 'not_taken', resolved_at: Time.current,
                                                         resolved_by_membership: actor)
    delete "#{path}/#{location.id}", headers: version_headers, as: :json
    expect(response).to have_http_status(:unprocessable_content)
    expect(outcome.reload).to be_persisted
    expect(location.reload).to be_persisted
  end

  it 'checks current permission before replaying a cached location edit' do
    observed = version_headers.merge('Idempotency-Key' => SecureRandom.uuid)
    patch "#{path}/#{location.id}", params: { location: { name: 'Updated storage' } }, headers: observed, as: :json
    expect(response).to have_http_status(:ok)
    actor = ApiSession.lookup_by_access_token(login.fetch('access_token')).household_membership
    actor.household.household_memberships.find_by!(account: accounts(:john_doe)).update!(role: :owner)
    actor.update!(role: :member)
    patch "#{path}/#{location.id}", params: { location: { name: 'Updated storage' } }, headers: observed, as: :json
    expect(response).to have_http_status(:forbidden)
  end
end
