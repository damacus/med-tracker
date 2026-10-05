# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Redirect back after form submission' do
  fixtures :accounts, :people, :locations, :location_memberships, :medications, :users, :dosages, :schedules

  let(:admin) { users(:admin) }
  let(:medication) { medications(:paracetamol) }
  let(:person) { people(:john) }

  before do
    sign_in(admin)
  end

  describe 'MedicationsController' do
    context 'when updating a medication with a return_to parameter' do
      it 'redirects back to the return_to path' do
        patch medication_path(medication),
              params: {
                medication: { name: 'Updated Name' },
                return_to: medications_url
              }

        expect(response).to redirect_to(medications_url)
      end
    end

    context 'when updating a medication without a return_to parameter' do
      it 'redirects to the medication show page as fallback' do
        patch medication_path(medication),
              params: { medication: { name: 'Updated Name' } }

        expect(response).to redirect_to(medication_path(medication))
      end
    end
  end

  describe 'PeopleController' do
    context 'when updating a person with a return_to parameter' do
      it 'redirects back to the return_to path' do
        patch person_path(person),
              params: {
                person: { name: 'Updated Name' },
                return_to: people_url
              }

        expect(response).to redirect_to(people_url)
      end
    end

    context 'when updating a person without a return_to parameter' do
      it 'redirects to the person show page as fallback' do
        patch person_path(person),
              params: { person: { name: 'Updated Name' } }

        expect(response).to redirect_to(person_path(person))
      end
    end
  end

  describe 'LocationsController' do
    let(:location) { locations(:home) }

    context 'when updating a location with a return_to parameter' do
      it 'redirects back to the return_to path' do
        patch location_path(location),
              params: {
                location: { name: 'Updated Home' },
                return_to: locations_url
              }

        expect(response).to redirect_to(locations_url)
      end
    end

    context 'when updating a location without a return_to parameter' do
      it 'redirects to the location show page as fallback' do
        patch location_path(location),
              params: { location: { name: 'Updated Home' } }

        expect(response).to redirect_to(location_path(location))
      end
    end
  end
end
