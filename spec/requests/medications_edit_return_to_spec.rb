# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medications edit return_to sanitization' do
  fixtures :accounts, :people, :users, :locations, :medications

  let(:admin) { users(:admin) }
  let(:medication) { medications(:paracetamol) }

  before { sign_in(admin) }

  describe 'GET /medications/:id/edit' do
    it 'preserves a safe internal return_to path' do
      get edit_medication_path(medication, return_to: '/medications')
      expect(response.body).to include('href="/medications"')
    end

    it 'strips an external return_to url from rendered links' do
      get edit_medication_path(medication, return_to: 'https://evil.com/phish')
      expect(response.body).not_to include('evil.com')
    end

    it 'strips a javascript: return_to scheme from rendered links' do
      get edit_medication_path(medication, return_to: 'javascript:alert(1)')
      expect(response.body).not_to include('javascript:alert')
    end
  end
end
