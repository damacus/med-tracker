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
           params: { medication: { name: '', description: 'A test', dose_amount: 10, dose_unit: 'mg' } }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('can&#39;t be blank')
    end

    it 'displays error summary at the top of the form' do
      post medications_path,
           params: { medication: { name: '', description: 'A test', dose_amount: 10, dose_unit: 'mg' } }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('role="alert"')
      expect(response.body).to include('error')
    end

    it 'adds error styling to fields with errors' do
      post medications_path,
           params: { medication: { name: '', description: 'A test', dose_amount: 10, dose_unit: 'mg' } }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('border-destructive')
    end

    it 'associates field errors with invalid inputs' do
      post medications_path,
           params: { medication: { name: '', description: 'A test', dose_amount: 10, dose_unit: 'mg' } }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('id="medication_name_error"')
      expect(response.body).to include('aria-invalid')
      expect(response.body).to include('aria-describedby="medication_name_error"')
    end

    it 'rejects zero dosage amount' do
      post medications_path,
           params: { medication: { name: 'Bad Dosage', description: 'A test', dose_amount: 0,
                                   dose_unit: 'tablet' } }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('must be greater than 0')
    end

    it 'renders validation messages in Spanish' do
      I18n.with_locale(:es) do
        post medications_path,
             params: { medication: { name: '', description: 'Prueba', dose_amount: 10, dose_unit: 'mg' } }
      end

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('no puede estar en blanco')
    end

    it 'renders validation messages in Welsh' do
      I18n.with_locale(:cy) do
        post medications_path,
             params: { medication: { name: '', description: 'Prawf', dose_amount: 10, dose_unit: 'mg' } }
      end

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('ni all fod yn wag')
    end

    it 'renders the date validation message in Spanish' do
      message = I18n.with_locale(:es) { I18n.t('errors.messages.on_or_after_start_date') }

      expect(message).to eq('debe ser igual o posterior a la fecha de inicio')
    end
  end
end
