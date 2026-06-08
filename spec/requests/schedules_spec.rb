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

  describe 'PATCH /people/:person_id/schedules/:id/stop' do
    before do
      sign_in(users(:admin))
    end

    it 'stops the schedule without deleting historical medication takes' do
      schedule = schedules(:john_paracetamol)
      MedicationTake.create!(
        schedule: schedule,
        taken_at: 1.day.ago,
        dose_amount: schedule.dose_amount,
        dose_unit: schedule.dose_unit
      )

      expect do
        patch stop_person_schedule_path(schedule.person, schedule),
              params: { return_to: medication_path(schedule.medication) }
      end.not_to change(MedicationTake, :count)

      expect(schedule.reload.stopped_on).to eq(Time.zone.today)
      expect(response).to redirect_to(medication_path(schedule.medication))
      expect(flash[:notice]).to eq('Schedule stopped')
    end
  end
end
