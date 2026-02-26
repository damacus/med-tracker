# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication Timing Restrictions' do
  fixtures :accounts, :people, :locations, :medications, :users, :dosages, :schedules, :carer_relationships

  # Use carer account (doesn't require 2FA)
  let(:carer_account) { accounts(:carer) }
  # Use child_patient who has carer_person as carer
  let(:person) { people(:child_patient) }
  let(:medication) { medications(:paracetamol) }

  before do
    # Login as carer (doesn't require 2FA)
    post '/login', params: { email: carer_account.email, password: 'password' }
  end

  describe 'POST /people/:person_id/schedules/:id/take_medication' do
    let(:schedule) do
      Schedule.create!(
        person: person,
        medication: medication,
        dosage: dosages(:paracetamol_adult),
        start_date: Time.zone.today - 1.day,
        end_date: Time.zone.today + 30.days,
        max_daily_doses: 2,
        min_hours_between_doses: 4
      )
    end

    context 'when timing restrictions allow taking medication' do
      it 'creates a medication take' do
        expect do
          post take_medication_person_schedule_path(person, schedule)
        end.to change(MedicationTake, :count).by(1)

        expect(response).to redirect_to(person_path(person))
        expect(flash[:notice]).to include('Medication taken')
      end
    end

    context 'when max daily doses reached' do
      before do
        2.times do
          MedicationTake.create!(
            schedule: schedule,
            taken_at: Time.current,
            amount_ml: 10
          )
        end
      end

      it 'does not create a medication take' do
        expect do
          post take_medication_person_schedule_path(person, schedule)
        end.not_to change(MedicationTake, :count)

        expect(response).to redirect_to(person_path(person))
        expect(flash[:alert]).to include('Cannot take medication')
      end
    end

    context 'when minimum hours between doses not met' do
      before do
        MedicationTake.create!(
          schedule: schedule,
          taken_at: 1.hour.ago,
          amount_ml: 10
        )
      end

      it 'does not create a medication take' do
        expect do
          post take_medication_person_schedule_path(person, schedule)
        end.not_to change(MedicationTake, :count)

        expect(response).to redirect_to(person_path(person))
        expect(flash[:alert]).to include('Cannot take medication')
      end
    end
  end

  describe 'POST /people/:person_id/person_medications/:id/take_medication' do
    let(:person_medication) do
      PersonMedication.create!(
        person: person,
        medication: medication,
        max_daily_doses: 2,
        min_hours_between_doses: 4
      )
    end

    context 'when timing restrictions allow taking medication' do
      it 'creates a medication take' do
        expect do
          post take_medication_person_person_medication_path(person, person_medication)
        end.to change(MedicationTake, :count).by(1)

        expect(response).to redirect_to(person_path(person))
        expect(flash[:notice]).to include('Medication taken')
      end
    end

    context 'when max daily doses reached' do
      before do
        2.times do
          MedicationTake.create!(
            person_medication: person_medication,
            taken_at: Time.current,
            amount_ml: 10
          )
        end
      end

      it 'does not create a medication take' do
        expect do
          post take_medication_person_person_medication_path(person, person_medication)
        end.not_to change(MedicationTake, :count)

        expect(response).to redirect_to(person_path(person))
        expect(flash[:alert]).to include('Cannot take medication')
      end
    end

    context 'when minimum hours between doses not met' do
      before do
        MedicationTake.create!(
          person_medication: person_medication,
          taken_at: 1.hour.ago,
          amount_ml: 10
        )
      end

      it 'does not create a medication take' do
        expect do
          post take_medication_person_person_medication_path(person, person_medication)
        end.not_to change(MedicationTake, :count)

        expect(response).to redirect_to(person_path(person))
        expect(flash[:alert]).to include('Cannot take medication')
      end
    end
  end
end
