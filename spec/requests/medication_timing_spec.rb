# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication Timing Restrictions' do
  fixtures :accounts, :people, :locations, :medications, :users, :dosages, :schedules, :carer_relationships,
           :location_memberships

  # Use carer account (doesn't require 2FA)
  let(:carer_account) { accounts(:carer) }
  # Use child_patient who has carer_person as carer
  let(:person) { people(:child_patient) }
  let(:medication) { medications(:paracetamol) }

  before do
    # Login as carer (doesn't require 2FA)
    sign_in(users(:carer))
  end

  describe 'POST /people/:person_id/schedules/:id/take_medication' do
    let(:schedule) do
      Schedule.create!(
        person: person,
        medication: medication,
        dose_amount: 1000,
        dose_unit: 'mg',
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

      it 'records the submitted historical taken_at timestamp' do
        submitted_time = Time.zone.local(2026, 4, 27, 8, 30)

        travel_to(Time.zone.local(2026, 4, 28, 12, 0)) do
          expect do
            post take_medication_person_schedule_path(person, schedule),
                 params: { medication_take: { taken_at: submitted_time.strftime('%Y-%m-%dT%H:%M') } }
          end.to change(MedicationTake, :count).by(1)
        end

        expect(MedicationTake.order(:id).last.taken_at).to be_within(1.second).of(submitted_time)
      end
    end

    context 'when submitted taken_at is invalid' do
      it 'does not create a medication take' do
        expect do
          post take_medication_person_schedule_path(person, schedule),
               params: { medication_take: { taken_at: 'not-a-date' } }
        end.not_to change(MedicationTake, :count)

        expect(response).to redirect_to(person_path(person))
        expect(flash[:alert]).to include('valid dose time')
      end
    end

    context 'when submitted taken_at is more than an hour in the future' do
      it 'does not create a medication take' do
        travel_to(Time.zone.local(2026, 4, 28, 12, 0)) do
          expect do
            post take_medication_person_schedule_path(person, schedule),
                 params: { medication_take: { taken_at: 61.minutes.from_now.strftime('%Y-%m-%dT%H:%M') } }
          end.not_to change(MedicationTake, :count)
        end

        expect(response).to redirect_to(person_path(person))
        expect(flash[:alert]).to include('future')
      end
    end

    context 'when submitted taken_at is within the 60 minute future tolerance' do
      it 'creates a medication take' do
        travel_to(Time.zone.local(2026, 4, 28, 12, 0)) do
          expect do
            post take_medication_person_schedule_path(person, schedule),
                 params: { medication_take: { taken_at: 30.minutes.from_now.strftime('%Y-%m-%dT%H:%M') } }
          end.to change(MedicationTake, :count).by(1)
        end
      end
    end

    context 'when submitted taken_at is a HH:MM time-only payload' do
      it 'records the dose against today at the submitted time' do
        travel_to(Time.zone.local(2026, 4, 28, 14, 0)) do
          expect do
            post take_medication_person_schedule_path(person, schedule),
                 params: { medication_take: { taken_at: '08:30' } }
          end.to change(MedicationTake, :count).by(1)

          expect(MedicationTake.order(:id).last.taken_at)
            .to be_within(1.second).of(Time.zone.local(2026, 4, 28, 8, 30))
        end
      end
    end

    context 'when max daily doses reached' do
      before do
        2.times do
          MedicationTake.create!(
            schedule: schedule,
            taken_at: Time.current,
            dose_amount: 10
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

      it 'allows another dose after the local midnight reset' do
        travel_to Time.zone.local(2026, 4, 29, 0, 5) do
          schedule.update!(min_hours_between_doses: nil)
          2.times do |index|
            MedicationTake.create!(
              schedule: schedule,
              taken_at: Time.zone.local(2026, 4, 28, 23, 30) + index.minutes,
              dose_amount: 10
            )
          end

          expect do
            post take_medication_person_schedule_path(person, schedule)
          end.to change(MedicationTake, :count).by(1)
        end

        expect(response).to redirect_to(person_path(person))
        expect(flash[:notice]).to include('Medication taken')
      end
    end

    context 'when minimum hours between doses not met' do
      before do
        MedicationTake.create!(
          schedule: schedule,
          taken_at: 1.hour.ago,
          dose_amount: 10
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

    context 'when another active prescription for the same medication sets a stricter limit' do
      let!(:other_schedule) do
        Schedule.create!(
          person: person,
          medication: medication,
          dose_amount: 1000,
          dose_unit: 'mg',
          start_date: Time.zone.today - 1.day,
          end_date: Time.zone.today + 30.days,
          max_daily_doses: 1,
          min_hours_between_doses: nil
        )
      end

      before do
        MedicationTake.create!(
          schedule: other_schedule,
          taken_at: 1.hour.ago,
          dose_amount: 1000,
          dose_unit: 'mg',
          taken_from_medication: medication,
          taken_from_location: medication.location
        )
      end

      it 'does not create a medication take and explains the overlapping prescription limit' do
        expect do
          post take_medication_person_schedule_path(person, schedule)
        end.not_to change(MedicationTake, :count)

        expect(response).to redirect_to(person_path(person))
        expect(flash[:alert]).to include('another active prescription')
      end
    end

    context 'when schedule dose is invalid' do
      it 'does not create a medication take and shows a friendly alert' do
        expect do
          post take_medication_person_schedule_path(person, schedule), params: { dose_amount: 0 }
        end.not_to change(MedicationTake, :count)

        expect(response).to redirect_to(person_path(person))
        expect(flash[:alert]).to include('Invalid dose configured')
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

      it 'records the submitted historical taken_at timestamp' do
        submitted_time = Time.zone.local(2026, 4, 27, 9, 15)

        travel_to(Time.zone.local(2026, 4, 28, 12, 0)) do
          expect do
            post take_medication_person_person_medication_path(person, person_medication),
                 params: { medication_take: { taken_at: submitted_time.strftime('%Y-%m-%dT%H:%M') } }
          end.to change(MedicationTake, :count).by(1)
        end

        expect(MedicationTake.order(:id).last.taken_at).to be_within(1.second).of(submitted_time)
      end
    end

    context 'when submitted taken_at is invalid' do
      it 'does not create a medication take' do
        expect do
          post take_medication_person_person_medication_path(person, person_medication),
               params: { medication_take: { taken_at: 'not-a-date' } }
        end.not_to change(MedicationTake, :count)

        expect(response).to redirect_to(person_path(person))
        expect(flash[:alert]).to include('valid dose time')
      end
    end

    context 'when submitted taken_at is more than an hour in the future' do
      it 'does not create a medication take' do
        travel_to(Time.zone.local(2026, 4, 28, 12, 0)) do
          expect do
            post take_medication_person_person_medication_path(person, person_medication),
                 params: { medication_take: { taken_at: 61.minutes.from_now.strftime('%Y-%m-%dT%H:%M') } }
          end.not_to change(MedicationTake, :count)
        end

        expect(response).to redirect_to(person_path(person))
        expect(flash[:alert]).to include('future')
      end
    end

    context 'when submitted taken_at is within the 60 minute future tolerance' do
      it 'creates a medication take' do
        travel_to(Time.zone.local(2026, 4, 28, 12, 0)) do
          expect do
            post take_medication_person_person_medication_path(person, person_medication),
                 params: { medication_take: { taken_at: 30.minutes.from_now.strftime('%Y-%m-%dT%H:%M') } }
          end.to change(MedicationTake, :count).by(1)
        end
      end
    end

    context 'when max daily doses reached' do
      before do
        2.times do
          MedicationTake.create!(
            person_medication: person_medication,
            taken_at: Time.current,
            dose_amount: 10
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
          dose_amount: 10
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

    context 'when medication dose is invalid' do
      it 'does not create a medication take and shows a friendly alert' do
        expect do
          post take_medication_person_person_medication_path(person, person_medication), params: { dose_amount: 0 }
        end.not_to change(MedicationTake, :count)

        expect(response).to redirect_to(person_path(person))
        expect(flash[:alert]).to include('Invalid dose configured')
      end
    end
  end
end
