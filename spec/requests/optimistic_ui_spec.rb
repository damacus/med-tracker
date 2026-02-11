# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Optimistic UI updates for take medicine buttons' do
  fixtures :accounts, :people, :medicines, :users, :dosages, :prescriptions, :carer_relationships

  let(:carer_account) { accounts(:carer) }
  let(:person) { people(:child_patient) }
  let(:medicine) { medicines(:paracetamol) }

  before do
    post '/login', params: { email: carer_account.email, password: 'password' }
  end

  describe 'prescription card' do
    before do
      Prescription.create!(
        person: person,
        medicine: medicine,
        dosage: dosages(:paracetamol_adult),
        start_date: Time.zone.today - 1.day,
        end_date: Time.zone.today + 30.days,
        max_daily_doses: 4,
        min_hours_between_doses: 1
      )
    end

    it 'renders take button with optimistic-take controller data attributes' do
      get person_path(person)

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('data-controller="optimistic-take"')
      expect(response.body).to include('data-optimistic-take-target="button"')
    end
  end

  describe 'person medicine card' do
    before do
      PersonMedicine.create!(
        person: person,
        medicine: medicine,
        max_daily_doses: 4,
        min_hours_between_doses: 1
      )
    end

    it 'renders take button with optimistic-take controller data attributes' do
      get person_path(person)

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('data-controller="optimistic-take"')
      expect(response.body).to include('data-optimistic-take-target="button"')
    end
  end
end
