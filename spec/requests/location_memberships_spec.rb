# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'LocationMemberships' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :households

  let(:admin) { users(:admin) }
  let(:location) { locations(:school) }
  let(:person) { people(:john) }

  before { sign_in(admin) }

  describe 'POST /locations/:location_id/location_memberships' do
    context 'with valid params' do
      it 'creates a new LocationMembership and redirects' do
        expect do
          post location_location_memberships_path(location),
               params: { location_membership: { person_id: person.id } }
        end.to change(LocationMembership, :count).by(1)

        expect(response).to redirect_to(location)
        expect(flash[:notice]).to be_present
      end
    end

    context 'when creation fails due to invalid params' do
      before do
        LocationMembership.create!(location: location, person: person)
      end

      it 'does not create a membership and redirects with an alert' do
        expect do
          post location_location_memberships_path(location),
               params: { location_membership: { person_id: person.id } }
        end.not_to change(LocationMembership, :count)

        expect(response).to redirect_to(location)
        expect(flash[:alert]).to be_present
      end
    end

    context 'when user is unauthorized' do
      let(:carer) { users(:carer) }

      before { sign_in(carer) }

      it 'redirects to root path' do
        post location_location_memberships_path(location),
             params: { location_membership: { person_id: person.id } }

        expect(response).to redirect_to(root_path)
        expect(flash[:alert]).to be_present
      end
    end
  end

  describe 'DELETE /locations/:location_id/location_memberships/:id' do
    let!(:membership) { LocationMembership.create!(location: location, person: person) }

    context 'when successful' do
      it 'destroys the membership and redirects' do
        expect do
          delete location_location_membership_path(location, membership)
        end.to change(LocationMembership, :count).by(-1)

        expect(response).to redirect_to(location)
        expect(flash[:notice]).to be_present
      end
    end

    context 'when destruction fails' do
      it 'redirects with an alert' do
        allow_any_instance_of(LocationMembership).to receive(:destroy).and_return(false) # rubocop:disable RSpec/AnyInstance

        delete location_location_membership_path(location, membership)

        expect(response).to redirect_to(location)
        expect(flash[:alert]).to be_present
      end
    end

    context 'when user is unauthorized' do
      let(:carer) { users(:carer) }

      before { sign_in(carer) }

      it 'redirects to root path' do
        delete location_location_membership_path(location, membership)

        expect(response).to redirect_to(root_path)
        expect(flash[:alert]).to be_present
      end
    end
  end
end
