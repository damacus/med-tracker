require 'rails_helper'

RSpec.describe 'API v1 dose outcome person access' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  %i[admin doctor nurse john carer parent].each do |role|
    it "allows #{role} to record only while their person grant permits recording" do
      actor = users(role)
      login = api_login(actor)
      membership = actor.person.account.first_active_household_membership
      source, grant = recording_source(actor, membership, self_care: role == :john)
      path = "/api/v1/households/#{membership.household_id}/schedules/#{source.id}/dose_occurrences/not_taken"
      headers = api_auth_headers(login.fetch('access_token'))
      key = MedicationAdministration::OccurrenceProjection.new(
        source: source, start_date: Date.current, end_date: Date.current
      ).call.first.key

      post path, params: { dose_occurrence: { key: key } }, headers: headers, as: :json
      expect(response).to have_http_status(:ok)
      grant.update!(access_level: :view)
      post path, params: { dose_occurrence: { key: key } }, headers: headers, as: :json
      expect(response).to have_http_status(:forbidden)
    end
  end

  def recording_source(actor, membership, self_care:)
    person = self_care ? actor.person : create(:person, household: membership.household)
    grant = PersonAccessGrant.find_or_initialize_by(household_membership: membership, person: person)
    grant.update!(household: membership.household, access_level: :record, relationship_type: :family_member)
    medication = create(:medication, household: membership.household)
    source = create(:schedule, household: membership.household, person: person, medication: medication,
                               frequency: 'Daily', start_date: Date.current, max_daily_doses: 1)
    [source, grant]
  end
end
