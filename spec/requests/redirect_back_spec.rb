# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Redirect back after form submission' do
  fixtures :accounts, :people, :locations, :location_memberships, :medicines, :users, :dosages, :prescriptions

  let(:admin) { users(:admin) }
  let(:medicine) { medicines(:paracetamol) }
  let(:person) { people(:john) }

  before do
    sign_in(admin)
  end

  describe 'MedicinesController' do
    context 'when updating a medicine with a return_to parameter' do
      it 'redirects back to the return_to path' do
        patch medicine_path(medicine),
              params: {
                medicine: { name: 'Updated Name' },
                return_to: medicines_url
              }

        expect(response).to redirect_to(medicines_url)
      end
    end

    context 'when updating a medicine without a return_to parameter' do
      it 'redirects to the medicine show page as fallback' do
        patch medicine_path(medicine),
              params: { medicine: { name: 'Updated Name' } }

        expect(response).to redirect_to(medicine_path(medicine))
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
