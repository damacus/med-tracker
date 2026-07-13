# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'People show card actions with turbo_stream' do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules, :carer_relationships

  before { sign_in(users(:admin)) }

  describe 'DELETE /people/:person_id/schedules/:id' do
    it 'returns turbo_stream and updates person show container and flash' do
      person = people(:john)
      schedule = schedules(:john_paracetamol)
      take = create(:medication_take, :for_schedule, schedule: schedule)

      delete person_schedule_path(person, schedule), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"#{household_dom_target("person_show_#{person.id}")}\"")
      expect(response.body).to include('target="flash"')
      expect(schedule.reload.retired_at).to be_present
      expect(MedicationTake.exists?(take.id)).to be(true)
      expect(PersonShowQuery.new(person: person).call.schedules).not_to include(schedule)
    end
  end

  describe 'DELETE /people/:person_id/person_medications/:id' do
    it 'returns turbo_stream and updates person show container and flash' do
      person = people(:child_user_person)
      person_medication = PersonMedication.create!(person: person, medication: medications(:vitamin_d))
      take = create(:medication_take, :for_person_medication, person_medication: person_medication)

      delete person_person_medication_path(person, person_medication),
             headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"#{household_dom_target("person_show_#{person.id}")}\"")
      expect(response.body).to include('target="flash"')
      expect(person_medication.reload.retired_at).to be_present
      expect(MedicationTake.exists?(take.id)).to be(true)
      expect(PersonShowQuery.new(person: person).call.person_medications).not_to include(person_medication)
    end
  end
end
