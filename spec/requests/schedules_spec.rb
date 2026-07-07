# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Schedules' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules,
           :carer_relationships

  describe 'GET /schedules' do
    context 'when not authenticated' do
      it 'redirects to login' do
        get schedules_path

        expect(response).to redirect_to(login_path)
      end
    end

    context 'when signed in as a carer' do
      before do
        sign_in(users(:carer))
      end

      it 'returns success and only displays active schedules in scope' do
        inactive_schedule = Schedule.create!(
          person: people(:child_patient),
          medication: medications(:ibuprofen),
          dose_amount: 200,
          dose_unit: 'mg',
          frequency: 'Inactive',
          start_date: 10.days.ago.to_date,
          end_date: 1.day.ago.to_date
        )

        get schedules_path

        expect(response).to have_http_status(:ok)
        expect(response.body).to include('Active Schedules')
        expect(response.body).to include(people(:child_patient).name)
        expect(response.body).to include(people(:child_user_person).name)
        expect(response.body).not_to include(people(:john).name)
        expect(response.body).not_to include(inactive_schedule.frequency)
      end
    end

    context 'when signed in as a minor' do
      before do
        sign_in(users(:minor_patient_user))
      end

      it 'rejects access' do
        get schedules_path

        expect(response).to redirect_to(root_path)
        expect(flash[:alert]).to eq('You are not authorized to perform this action.')
      end
    end
  end

  describe 'GET /schedules/new' do
    before do
      sign_in(users(:admin))
    end

    it 'redirects direct schedule creation to the workflow' do
      get new_schedule_path

      expect(response).to redirect_to(schedules_workflow_path)
    end
  end

  describe 'POST /people/:person_id/schedules' do
    before { sign_in(users(:admin)) }

    it 'creates a schedule and redirects to the person page' do
      person = people(:john)
      medication = medications(:paracetamol)

      expect do
        post person_schedules_path(person),
             params: {
               schedule: {
                 medication_id: medication.id,
                 dose_amount: '500',
                 dose_unit: 'mg',
                 frequency: 'Daily',
                 start_date: Time.zone.today.to_s,
                 end_date: 1.month.from_now.to_date.to_s
               }
             }
      end.to change(Schedule, :count).by(1)

      expect(response).to redirect_to(person_path(person))
    end
  end

  describe 'PATCH /people/:person_id/schedules/:id' do
    before { sign_in(users(:admin)) }

    it 'updates a schedule and redirects to the person page' do
      schedule = schedules(:john_paracetamol)

      patch person_schedule_path(schedule.person, schedule),
            params: { schedule: { frequency: 'Twice daily' } }

      expect(response).to redirect_to(person_path(schedule.person))
      expect(schedule.reload.frequency).to eq('Twice daily')
    end
  end

  describe 'PATCH /people/:person_id/schedules/:id/pause' do
    let(:schedule) { schedules(:child_schedule) }
    let(:person) { schedule.person }

    context 'when signed in as a parent of the linked child' do
      before { sign_in(users(:parent)) }

      it 'pauses the schedule and redirects to the person page' do
        patch pause_person_schedule_path(person, schedule)

        expect(response).to redirect_to(person_path(person))
        expect(schedule.reload).to be_paused
      end

      it 'returns a turbo stream refresh for the person show container' do
        patch pause_person_schedule_path(person, schedule),
              headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

        expect(response).to have_http_status(:ok)
        expect(response.media_type).to eq('text/vnd.turbo-stream.html')
        expect(response.body).to include("target=\"#{household_dom_target("person_show_#{person.id}")}\"")
        expect(schedule.reload).to be_paused
      end
    end

    context 'when signed in as a carer without manage access' do
      before { sign_in(users(:carer)) }

      it 'does not pause the schedule' do
        carer_schedule = schedules(:patient_schedule)

        patch pause_person_schedule_path(carer_schedule.person, carer_schedule)

        expect(response).to redirect_to(root_path)
        expect(carer_schedule.reload).not_to be_paused
      end
    end
  end

  describe 'PATCH /people/:person_id/schedules/:id/resume' do
    let(:schedule) { schedules(:child_schedule) }

    before do
      sign_in(users(:parent))
      schedule.pause!
    end

    it 'resumes the schedule and redirects to the person page' do
      patch resume_person_schedule_path(schedule.person, schedule)

      expect(response).to redirect_to(person_path(schedule.person))
      expect(schedule.reload).not_to be_paused
    end
  end
end
