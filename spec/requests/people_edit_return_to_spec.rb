# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'People edit return_to sanitization' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :carer_relationships

  let(:user) { users(:admin) }
  let(:person) { people(:jane) }

  before { sign_in(user) }

  describe 'GET /people/:id/edit' do
    it 'preserves a safe internal return_to path' do
      get edit_person_path(person, return_to: '/people')
      expect(response.body).to include('href="/people"')
    end

    it 'strips an external return_to url from rendered links' do
      get edit_person_path(person, return_to: 'https://evil.com/phish')
      expect(response.body).not_to include('evil.com')
    end

    it 'strips a javascript: return_to scheme from rendered links' do
      get edit_person_path(person, return_to: 'javascript:alert(1)')
      expect(response.body).not_to include('javascript:alert')
    end
  end
end
