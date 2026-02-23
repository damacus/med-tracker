# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Redirect back after form submission' do
  fixtures :accounts, :people, :locations, :medicines, :users, :dosages, :prescriptions

  let(:admin) { users(:admin) }
  let(:medicine) { medicines(:paracetamol) }
  let(:person) { people(:john) }

  before do
    sign_in(admin)
  end

  describe 'MedicinesController' do
    context 'when updating a medicine with a referrer' do
      it 'redirects back to the referrer' do
        patch medicine_path(medicine),
              params: { medicine: { name: 'Updated Name' } },
              headers: { 'HTTP_REFERER' => medicines_url }

        expect(response).to redirect_to(medicines_url)
      end
    end

    context 'when updating a medicine without a referrer' do
      it 'redirects to the medicine show page as fallback' do
        patch medicine_path(medicine),
              params: { medicine: { name: 'Updated Name' } }

        expect(response).to redirect_to(medicine_path(medicine))
      end
    end
  end

  describe 'PeopleController' do
    context 'when updating a person with a referrer' do
      it 'redirects back to the referrer' do
        patch person_path(person),
              params: { person: { name: 'Updated Name' } },
              headers: { 'HTTP_REFERER' => people_url }

        expect(response).to redirect_to(people_url)
      end
    end

    context 'when updating a person without a referrer' do
      it 'redirects to the person show page as fallback' do
        patch person_path(person),
              params: { person: { name: 'Updated Name' } }

        expect(response).to redirect_to(person_path(person))
      end
    end
  end
end
