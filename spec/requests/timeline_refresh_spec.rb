# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Timeline refresh after taking medicine' do
  fixtures :accounts, :people, :medicines, :users, :dosages, :prescriptions, :carer_relationships

  let(:carer_account) { accounts(:carer) }
  let(:person) { people(:child_patient) }
  let(:medicine) { medicines(:paracetamol) }

  before do
    post '/login', params: { email: carer_account.email, password: 'password' }
  end

  describe 'POST take_medicine for a prescription (turbo_stream)' do
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

    it 'includes a turbo-stream replace for the timeline item' do
      post take_medicine_person_prescription_path(person, prescription),
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expected_id = "timeline_prescription_#{prescription.id}"
      expect(response.body).to include(expected_id)
    end

    it 'includes a turbo-stream replace for the prescription card' do
      post take_medicine_person_prescription_path(person, prescription),
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expected_id = "prescription_#{prescription.id}"
      expect(response.body).to include(expected_id)
    end

    it 'marks the timeline item as taken in the response' do
      post take_medicine_person_prescription_path(person, prescription),
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response.body).to include('Taken')
    end
  end

  describe 'POST take_medicine for a person_medicine (turbo_stream)' do
    let(:person_medicine) do
      PersonMedicine.create!(
        person: person,
        medicine: medicine,
        max_daily_doses: 4,
        min_hours_between_doses: 1
      )
    end

    it 'includes a turbo-stream replace for the timeline item' do
      post take_medicine_person_person_medicine_path(person, person_medicine),
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expected_id = "timeline_person_medicine_#{person_medicine.id}"
      expect(response.body).to include(expected_id)
    end

    it 'includes a turbo-stream replace for the person_medicine card' do
      post take_medicine_person_person_medicine_path(person, person_medicine),
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expected_id = "person_medicine_#{person_medicine.id}"
      expect(response.body).to include(expected_id)
    end

    it 'marks the timeline item as taken in the response' do
      post take_medicine_person_person_medicine_path(person, person_medicine),
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response.body).to include('Taken')
    end
  end
end
