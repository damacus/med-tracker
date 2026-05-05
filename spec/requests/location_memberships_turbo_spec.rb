# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Location memberships turbo streams' do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:admin) { users(:admin) }
  let(:location) { locations(:school) }

  before { sign_in(admin) }

  describe 'POST /locations/:location_id/location_memberships' do
    it 'returns turbo_stream and replaces the location show container and flash' do
      person = people(:john)

      expect do
        post location_location_memberships_path(location),
             params: { location_membership: { person_id: person.id } },
             headers: { 'Accept' => 'text/vnd.turbo-stream.html' }
      end.to change(LocationMembership, :count).by(1)

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"location_show_#{location.id}\"")
      expect(response.body).to include(person.name)
      expect(response.body).to include('target="flash"')
    end
  end

  describe 'DELETE /locations/:location_id/location_memberships/:id' do
    it 'returns turbo_stream and replaces the location show container and flash' do
      membership = location_memberships(:jane_school)

      expect do
        delete location_location_membership_path(location, membership),
               headers: { 'Accept' => 'text/vnd.turbo-stream.html' }
      end.to change(LocationMembership, :count).by(-1)

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"location_show_#{location.id}\"")
      expect(response.body).to include('target="flash"')
    end
  end
end
