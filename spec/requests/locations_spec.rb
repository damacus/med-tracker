# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Locations' do
  fixtures :all

  let(:admin) { users(:admin) }

  describe 'GET /locations' do
    context 'when not authenticated' do
      it 'redirects to login' do
        get locations_path
        expect(response).to redirect_to(login_path)
      end
    end

    context 'when authenticated as admin' do
      before { sign_in(admin) }

      it 'returns HTTP success' do
        get locations_path
        expect(response).to have_http_status(:success)
        expect(response.body).to include('id="locations_index"')
      end
    end

    context 'when authenticated as carer (unauthorized)' do
      let(:carer) { users(:carer) }

      before { sign_in(carer) }

      it 'returns success with empty list' do
        get locations_path
        expect(response).to have_http_status(:success)
      end
    end

    context 'when authenticated as carer with household locations' do
      let(:carer) { users(:carer) }

      before { sign_in(carer) }

      it 'renders only accessible locations' do
        foreign_location

        get locations_path
        expect(response.body).to include('Home')
        expect(response.body).not_to include('Foreign School')
      end
    end
  end

  describe 'GET /locations/:id' do
    let(:location) { locations(:home) }

    context 'when authenticated as admin' do
      before { sign_in(admin) }

      it 'returns HTTP success' do
        get location_path(location)
        expect(response).to have_http_status(:success)
        expect(response.body).to include("id=\"#{household_dom_target("location_show_#{location.id}")}\"")
      end
    end

    context 'when authenticated as carer for an accessible location' do
      let(:location) { household_location_named('Home') }
      let(:carer) { users(:carer) }

      before { sign_in(carer) }

      it 'returns HTTP success' do
        get location_path(location)
        expect(response).to have_http_status(:success)
      end
    end

    context 'when authenticated as carer for a foreign location' do
      let(:location) { foreign_location }
      let(:carer) { users(:carer) }

      before { sign_in(carer) }

      it 'returns not found through the scoped lookup' do
        get location_path(location)
        expect(response).to have_http_status(:not_found)
      end
    end
  end

  def foreign_location
    household = Household.create!(name: 'Foreign Location Household', slug: 'foreign-location-household')
    household.locations.create!(name: 'Foreign School', description: 'Foreign school location')
  end

  describe 'GET /locations/new' do
    context 'when authenticated as admin' do
      before { sign_in(admin) }

      it 'returns HTTP success' do
        get new_location_path
        expect(response).to have_http_status(:success)
      end
    end
  end

  describe 'GET /locations/:id/edit' do
    let(:location) { locations(:home) }

    context 'when authenticated as admin' do
      before { sign_in(admin) }

      it 'preserves a safe internal return_to path' do
        get edit_location_path(location, return_to: '/locations')
        expect(response.body).to include('href="/locations"')
      end

      it 'strips an external return_to url from rendered links' do
        get edit_location_path(location, return_to: 'https://evil.com/phish')
        expect(response.body).not_to include('evil.com')
      end

      it 'strips a javascript: return_to scheme from rendered links' do
        get edit_location_path(location, return_to: 'javascript:alert(1)')
        expect(response.body).not_to include('javascript:alert')
      end
    end
  end

  describe 'POST /locations' do
    context 'when authenticated as admin' do
      before { sign_in(admin) }

      it 'creates a location with valid params' do
        expect do
          post locations_path, params: { location: { name: 'Office', description: 'Work office' } }
        end.to change(Location, :count).by(1)

        expect(response).to redirect_to(location_path(Location.last))
      end

      it 'returns turbo_stream and replaces main content and flash on success' do
        expect do
          post locations_path,
               params: { location: { name: 'Turbo Office', description: 'Work office' } },
               headers: { 'Accept' => 'text/vnd.turbo-stream.html' }
        end.to change(Location, :count).by(1)

        expect(response).to have_http_status(:ok)
        expect(response.media_type).to eq('text/vnd.turbo-stream.html')
        expect(response.body).to include('target="main-content"')
        expect(response.body).to include("id=\"#{household_dom_target("location_show_#{Location.last.id}")}\"")
        expect(response.body).to include('target="flash"')
      end

      it 'renders form with invalid params' do
        post locations_path, params: { location: { name: '' } }
        expect(response).to have_http_status(:unprocessable_content)
      end
    end
  end

  describe 'PATCH /locations/:id' do
    let(:location) { locations(:home) }

    context 'when authenticated as admin' do
      before { sign_in(admin) }

      it 'updates the location' do
        patch location_path(location), params: { location: { name: 'Updated Home' } }
        expect(response).to redirect_to(location_path(location))
        expect(location.reload.name).to eq('Updated Home')
      end

      it 'returns turbo_stream and replaces main content and flash on success' do
        patch location_path(location),
              params: { location: { name: 'Turbo Updated Home' } },
              headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

        expect(response).to have_http_status(:ok)
        expect(response.media_type).to eq('text/vnd.turbo-stream.html')
        expect(response.body).to include('target="main-content"')
        expect(response.body).to include("id=\"#{household_dom_target("location_show_#{location.id}")}\"")
        expect(response.body).to include('Turbo Updated Home')
        expect(response.body).to include('target="flash"')
      end

      it 'renders form with invalid params' do
        patch location_path(location), params: { location: { name: '' } }
        expect(response).to have_http_status(:unprocessable_content)
      end
    end
  end

  describe 'DELETE /locations/:id' do
    let(:location) { locations(:grandmas) }

    context 'when authenticated as admin' do
      before { sign_in(admin) }

      it 'deletes the location' do
        expect do
          delete location_path(location)
        end.to change(Location, :count).by(-1)

        expect(response).to redirect_to(locations_path)
      end

      it 'returns turbo_stream and removes location targets and updates flash' do
        expect do
          delete location_path(location), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }
        end.to change(Location, :count).by(-1)

        expect(response).to have_http_status(:ok)
        expect(response.media_type).to eq('text/vnd.turbo-stream.html')
        expect(response.body).to include("target=\"#{household_dom_target("location_#{location.id}")}\"")
        expect(response.body).to include("target=\"#{household_dom_target("location_show_#{location.id}")}\"")
        expect(response.body).to include('target="flash"')
      end

      it 'preserves medication history and renders a Turbo validation error when deletion is restricted' do
        medication = create(:medication, household: location.household, location: location)
        schedule = create(:schedule, household: location.household, medication: medication)
        create(:medication_take, :for_schedule, household: location.household, schedule: schedule)

        delete location_path(location), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

        expect(response).to have_http_status(:unprocessable_content)
        expect(response.media_type).to eq('text/vnd.turbo-stream.html')
        expect(Location.exists?(location.id)).to be(true)
        expect(Medication.exists?(medication.id)).to be(true)
        expect(Schedule.exists?(schedule.id)).to be(true)
        expect(response.body).to include('target="flash"')
        expect(response.body).not_to include("target=\"#{household_dom_target("location_#{location.id}")}\"")
      end
    end

    context 'when authenticated as doctor (unauthorized to delete)' do
      let(:doctor) { users(:doctor) }

      before { sign_in(doctor) }

      it 'redirects with authorization error' do
        delete location_path(location)
        expect(response).to have_http_status(:found)
      end
    end
  end

  def household_location_named(name)
    household = Household.find_by!(slug: default_request_household_slug)
    household.locations.find_by!(name: name)
  end
end
