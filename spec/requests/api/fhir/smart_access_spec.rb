# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'SMART FHIR access' do
  fixtures :accounts, :people, :users

  let(:smart_context) do
    person = users(:jane).person
    account = person.account
    membership = person.household.household_memberships.find_or_create_by!(
      account: account,
      person: person
    ).tap do |record|
      record.update!(role: :member, status: :active)
    end
    access_grant = person.household.person_access_grants.find_or_initialize_by(
      household_membership: membership,
      person: person,
      revoked_at: nil
    )
    access_grant.update!(
      access_level: :manage,
      relationship_type: :self,
      granted_by_membership: membership
    )
    raw_token = 'smart-access-token'
    oauth_application = OauthApplication.create!(
      name: 'SMART FHIR client',
      client_id: SecureRandom.uuid,
      redirect_uri: 'https://client.example/callback',
      scopes: 'patient/Patient.rs patient/Medication.rs'
    )
    grant = OauthGrant.create!(
      account: account,
      oauth_application: oauth_application,
      household_membership: membership,
      person: person,
      permissions_version: membership.permissions_version,
      token_hash: OauthGrant.digest(raw_token),
      expires_in: 15.minutes.from_now,
      scopes: 'patient/Patient.rs'
    )

    { person: person, account: account, membership: membership, raw_token: raw_token, grant: grant }
  end

  it 'accepts a valid patient read scope' do
    get "/api/fhir/R4/Patient/#{person.portable_id}", headers: smart_headers

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('"resourceType":"Patient"', "\"id\":\"#{person.portable_id}\"")
  end

  it 'rejects a token without the requested resource scope' do
    get '/api/fhir/R4/Medication', headers: smart_headers

    expect(response).to have_http_status(:forbidden)
  end

  it 'rejects revoked grants' do
    grant.update!(revoked_at: Time.current)

    get "/api/fhir/R4/Patient/#{person.portable_id}", headers: smart_headers

    expect(response).to have_http_status(:unauthorized)
  end

  it 'rejects grants after their membership permissions change' do
    membership.update!(permissions_version: membership.permissions_version + 1)

    get "/api/fhir/R4/Patient/#{person.portable_id}", headers: smart_headers

    expect(response).to have_http_status(:unauthorized)
  end

  it 'rejects grants after their membership is revoked' do
    membership.update!(status: :revoked)

    get "/api/fhir/R4/Patient/#{person.portable_id}", headers: smart_headers

    expect(response).to have_http_status(:unauthorized)
  end

  it 'rejects grants for locked accounts' do
    allow(ApiAuthState).to receive(:locked_out?).with(account).and_return(true)

    get "/api/fhir/R4/Patient/#{person.portable_id}", headers: smart_headers

    expect(response).to have_http_status(:unauthorized)
  end

  it 'does not expose a patient from another household' do
    other_household = Household.create!(name: 'Other SMART household')
    other_person = other_household.people.create!(
      name: 'Other SMART patient',
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )

    get "/api/fhir/R4/Patient/#{other_person.portable_id}", headers: smart_headers

    expect(response).to have_http_status(:not_found)
  end

  def smart_headers
    { 'Authorization' => "Bearer #{smart_context.fetch(:raw_token)}" }
  end

  def person = smart_context.fetch(:person)
  def account = smart_context.fetch(:account)
  def membership = smart_context.fetch(:membership)
  def grant = smart_context.fetch(:grant)
end
