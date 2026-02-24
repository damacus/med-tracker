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
      before { post '/login', params: { email: admin.email_address, password: 'password' } }

      it 'returns HTTP success' do
        get locations_path
        expect(response).to have_http_status(:success)
      end
    end

    context 'when authenticated as carer (unauthorized)' do
      let(:carer) { users(:carer) }

      before { post '/login', params: { email: carer.email_address, password: 'password' } }

      it 'returns success with empty list' do
        get locations_path
        expect(response).to have_http_status(:success)
      end
    end
  end

  describe 'GET /locations/:id' do
    let(:location) { locations(:home) }

    context 'when authenticated as admin' do
      before { post '/login', params: { email: admin.email_address, password: 'password' } }

      it 'returns HTTP success' do
        get location_path(location)
        expect(response).to have_http_status(:success)
      end
    end
  end

  describe 'GET /locations/new' do
    context 'when authenticated as admin' do
      before { post '/login', params: { email: admin.email_address, password: 'password' } }

      it 'returns HTTP success' do
        get new_location_path
        expect(response).to have_http_status(:success)
      end
    end
  end

  describe 'POST /locations' do
    context 'when authenticated as admin' do
      before { post '/login', params: { email: admin.email_address, password: 'password' } }

      it 'creates a location with valid params' do
        expect do
          post locations_path, params: { location: { name: 'Office', description: 'Work office' } }
        end.to change(Location, :count).by(1)

        expect(response).to redirect_to(location_path(Location.last))
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
      before { post '/login', params: { email: admin.email_address, password: 'password' } }

      it 'updates the location' do
        patch location_path(location), params: { location: { name: 'Updated Home' } }
        expect(response).to redirect_to(location_path(location))
        expect(location.reload.name).to eq('Updated Home')
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
      before { post '/login', params: { email: admin.email_address, password: 'password' } }

      it 'destroys the location' do
        expect do
          delete location_path(location)
        end.to change(Location, :count).by(-1)

        expect(response).to redirect_to(locations_path)
      end
    end

    context 'when authenticated as doctor (unauthorized for destroy)' do
      let(:doctor) { users(:doctor) }

      before { post '/login', params: { email: doctor.email_address, password: 'password' } }

      it 'redirects with authorization error' do
        delete location_path(location)
        expect(response).to redirect_to(root_path)
      end
    end
  end
end
