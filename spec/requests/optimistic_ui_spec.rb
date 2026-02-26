# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Optimistic UI updates for take medication buttons' do
  fixtures :accounts, :people, :locations, :medications, :users, :dosages, :schedules, :carer_relationships

  let(:carer_account) { accounts(:carer) }
  let(:person) { people(:child_patient) }
  let(:medication) { medications(:paracetamol) }

  before do
    post '/login', params: { email: carer_account.email, password: 'password' }
  end

  describe 'schedule card' do
    before do
      Schedule.create!(
        person: person,
        medication: medication,
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

  describe 'person medication card' do
    before do
      PersonMedication.create!(
        person: person,
        medication: medication,
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
