# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Toast notifications for async actions' do
  fixtures :accounts, :people, :medicines, :users, :dosages, :prescriptions, :carer_relationships

  let(:carer_account) { accounts(:carer) }
  let(:person) { people(:child_patient) }
  let(:medicine) { medicines(:paracetamol) }

  before do
    post '/login', params: { email: carer_account.email, password: 'password' }
  end

  describe 'POST /people/:person_id/prescriptions/:id/take_medicine' do
    let(:prescription) do
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

    context 'with turbo_stream format' do
      it 'returns turbo_stream response with flash update on success' do
        post take_medicine_person_prescription_path(person, prescription),
             headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

        expect(response).to have_http_status(:ok)
        expect(response.media_type).to eq('text/vnd.turbo-stream.html')
        expect(response.body).to include('turbo-stream')
        expect(response.body).to include('flash')
      end

      it 'returns turbo_stream response with flash update on failure' do
        prescription.update!(max_daily_doses: 0)

        post take_medicine_person_prescription_path(person, prescription),
             headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

        expect(response).to have_http_status(:ok)
        expect(response.media_type).to eq('text/vnd.turbo-stream.html')
        expect(response.body).to include('turbo-stream')
        expect(response.body).to include('flash')
      end
    end

    context 'with html format' do
      it 'still redirects as before' do
        post take_medicine_person_prescription_path(person, prescription)

        expect(response).to redirect_to(person_path(person))
      end
    end
  end

  describe 'POST /people/:person_id/person_medicines/:id/take_medicine' do
    let(:person_medicine) do
      PersonMedicine.create!(
        person: person,
        medicine: medicine,
        max_daily_doses: 4,
        min_hours_between_doses: 1
      )
    end

    context 'with turbo_stream format' do
      it 'returns turbo_stream response with flash update on success' do
        post take_medicine_person_person_medicine_path(person, person_medicine),
             headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

        expect(response).to have_http_status(:ok)
        expect(response.media_type).to eq('text/vnd.turbo-stream.html')
        expect(response.body).to include('turbo-stream')
        expect(response.body).to include('flash')
      end
    end
  end
end
