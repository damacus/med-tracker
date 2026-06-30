# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Timeline refresh after taking medication' do
  fixtures :accounts, :people, :medications, :users, :dosages, :schedules, :carer_relationships

  let(:carer_account) { accounts(:carer) }
  let(:person) { people(:child_patient) }
  let(:medication) { medications(:paracetamol) }

  before do
    sign_in(users(:carer))
  end

  describe 'POST take_medication for a schedule (turbo_stream)' do
    let(:schedule) do
      Schedule.create!(
        person: person,
        medication: medication,
        dose_amount: 1000,
        dose_unit: 'mg',
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

    it 'does not include unauthorized same-medication timeline items in the response' do
      unauthorized_schedule = schedules(:john_paracetamol)
      child_paracetamol = Schedule.create!(
        person: person,
        medication: medication,
        dose_amount: 250,
        dose_unit: 'mg',
        start_date: Time.zone.today - 1.day,
        end_date: Time.zone.today + 30.days,
        max_daily_doses: 4,
        min_hours_between_doses: 1
      )

      scoped_schedules = SchedulePolicy::Scope.new(users(:carer), Schedule.all).resolve
      expect(scoped_schedules).not_to include(unauthorized_schedule)

      post take_medication_person_schedule_path(person, child_paracetamol),
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response.body).to include("timeline_schedule_#{child_paracetamol.id}")
      expect(response.body).not_to include("timeline_schedule_#{unauthorized_schedule.id}")
      expect(response.body).not_to include('John Doe')
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
