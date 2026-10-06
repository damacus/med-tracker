require 'rails_helper'

RSpec.describe 'API v1 location membership writes' do
  fixtures :accounts, :people, :users, :locations, :households, :location_memberships

  let(:login) { api_login(users(:admin)) }
  let(:headers) { api_auth_headers(login.fetch('access_token')) }
  let(:location) { Location.create!(household: users(:admin).person.household, name: 'API membership storage') }
  let(:person) { people(:john) }
  let(:path) do
    "/api/v1/households/#{login.dig('household', 'id')}/locations/#{location.portable_id}/location_memberships"
  end

  before { login }

  def assign
    post path, params: { location_membership: { person_id: person.portable_id } }, headers: headers, as: :json
  end

  it 'assigns a managed person by portable identity and replays without duplication' do
    expect { assign }.to change(LocationMembership, :count).by(1)
    expect(response).to have_http_status(:created)
    original = response.parsed_body.fetch('data')
    expect(original).to include('person_portable_id' => person.portable_id,
                                'location_portable_id' => location.portable_id)
    expect { assign }.not_to change(LocationMembership, :count)
    expect(response.parsed_body.fetch('data')).to eq(original)
    expect(location.location_memberships.sole.household_id).to eq(location.household_id)
  end

  it 'removes a membership only from the selected location' do
    assignment = LocationMembership.create!(person: person, location: location)
    delete "#{path}/#{assignment.id}", headers: headers, as: :json
    expect(response).to have_http_status(:no_content)
    expect(LocationMembership.exists?(assignment.id)).to be(false)
  end

  it 'rejects a view-only person for assignment and removal' do
    assignment = LocationMembership.create!(person: person, location: location)
    actor = ApiSession.lookup_by_access_token(login.fetch('access_token')).household_membership
    PersonAccessGrant.where(household_membership: actor, person: person).find_each do |grant|
      grant.update!(access_level: :view)
    end
    assign
    expect(response).to have_http_status(:forbidden)
    delete "#{path}/#{assignment.id}", headers: headers, as: :json
    expect(response).to have_http_status(:forbidden)
    expect(assignment.reload).to be_persisted
  end

  it 'checks person access again before replaying an idempotency-cached assignment' do
    headers['Idempotency-Key'] = SecureRandom.uuid
    assign
    expect(response).to have_http_status(:created)
    actor = ApiSession.lookup_by_access_token(login.fetch('access_token')).household_membership
    PersonAccessGrant.where(household_membership: actor, person: person).find_each(&:destroy!)
    assign
    expect(response).to have_http_status(:not_found)
    expect(response.body).not_to include(person.name, person.portable_id)
  end

  it 'rejects a foreign person without changing membership or grants' do
    foreign = create(:person, household: create(:household))
    expect do
      post path, params: { location_membership: { person_id: foreign.portable_id } }, headers: headers, as: :json
    end.not_to change(LocationMembership, :count)
    expect(response).to have_http_status(:not_found)
    expect(response.body).not_to include(foreign.name)
  end

  it 'does not remove a membership belonging to another location' do
    other = Location.create!(household: location.household, name: 'Other storage')
    assignment = LocationMembership.create!(person: person, location: other)
    delete "#{path}/#{assignment.id}", headers: headers, as: :json
    expect(response).to have_http_status(:not_found)
    expect(assignment.reload).to be_persisted
  end

  it 'requires permission for the location action even when the person is managed' do
    member = api_login(users(:jane))
    post path, params: { location_membership: { person_id: users(:jane).person.portable_id } },
               headers: api_auth_headers(member.fetch('access_token')), as: :json
    expect(response).to have_http_status(:forbidden)
    expect(location.location_memberships).to be_empty
  end
end
