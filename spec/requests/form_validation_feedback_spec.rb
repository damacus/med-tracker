# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Form inline validation feedback' do
  fixtures :accounts, :people, :locations, :medications, :users, :dosages, :schedules

  let(:admin) { users(:admin) }

  before do
    sign_in(admin)
  end

  describe 'MedicationsController with validation errors' do
    it 'displays inline error messages next to invalid fields' do
      post medications_path,
           params: { medication: { name: '', description: 'A test', dosage_amount: 10, dosage_unit: 'mg' } }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('can&#39;t be blank')
    end

    it 'displays error summary at the top of the form' do
      post medications_path,
           params: { medication: { name: '', description: 'A test', dosage_amount: 10, dosage_unit: 'mg' } }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('error')
    end

    it 'adds error styling to fields with errors' do
      post medications_path,
           params: { medication: { name: '', description: 'A test', dosage_amount: 10, dosage_unit: 'mg' } }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('border-destructive')
    end
  end
end
