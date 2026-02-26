# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Timeline refresh after taking medication' do
  fixtures :accounts, :people, :medications, :users, :dosages, :schedules, :carer_relationships

  let(:carer_account) { accounts(:carer) }
  let(:person) { people(:child_patient) }
  let(:medication) { medications(:paracetamol) }

  before do
    post '/login', params: { email: carer_account.email, password: 'password' }
  end

  describe 'POST take_medication for a schedule (turbo_stream)' do
    let(:schedule) do
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

    it 'includes a turbo-stream replace for the timeline item' do
      post take_medication_person_schedule_path(person, schedule),
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expected_id = "timeline_schedule_#{schedule.id}"
      expect(response.body).to include(expected_id)
    end

    it 'includes a turbo-stream replace for the schedule card' do
      post take_medication_person_schedule_path(person, schedule),
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expected_id = "schedule_#{schedule.id}"
      expect(response.body).to include(expected_id)
    end

    it 'marks the timeline item as taken in the response' do
      post take_medication_person_schedule_path(person, schedule),
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response.body).to include('Taken')
    end
  end

  describe 'POST take_medication for a person_medication (turbo_stream)' do
    let(:person_medication) do
      PersonMedication.create!(
        person: person,
        medication: medication,
        max_daily_doses: 4,
        min_hours_between_doses: 1
      )
    end

    it 'includes a turbo-stream replace for the timeline item' do
      post take_medication_person_person_medication_path(person, person_medication),
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expected_id = "timeline_person_medication_#{person_medication.id}"
      expect(response.body).to include(expected_id)
    end

    it 'includes a turbo-stream replace for the person_medication card' do
      post take_medication_person_person_medication_path(person, person_medication),
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expected_id = "person_medication_#{person_medication.id}"
      expect(response.body).to include(expected_id)
    end

    it 'marks the timeline item as taken in the response' do
      post take_medication_person_person_medication_path(person, person_medication),
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response.body).to include('Taken')
    end
  end
end
