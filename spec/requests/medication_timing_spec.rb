# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication Timing Restrictions' do
  fixtures :accounts, :people, :medicines, :users, :dosages, :prescriptions, :carer_relationships

  # Use carer account (doesn't require 2FA)
  let(:carer_account) { accounts(:carer) }
  # Use child_patient who has carer_person as carer
  let(:person) { people(:child_patient) }
  let(:medicine) { medicines(:paracetamol) }

  before do
    # Login as carer (doesn't require 2FA)
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
        max_daily_doses: 2,
        min_hours_between_doses: 4
      )
    end

    context 'when timing restrictions allow taking medicine' do
      it 'creates a medication take' do
        expect do
          post take_medicine_person_prescription_path(person, prescription)
        end.to change(MedicationTake, :count).by(1)

        expect(response).to redirect_to(person_path(person))
        expect(flash[:notice]).to include('Medicine taken')
      end
    end

    context 'when max daily doses reached' do
      before do
        2.times do
          MedicationTake.create!(
            prescription: prescription,
            taken_at: Time.current,
            amount_ml: 10
          )
        end
      end

      it 'does not create a medication take' do
        expect do
          post take_medicine_person_prescription_path(person, prescription)
        end.not_to change(MedicationTake, :count)

        expect(response).to redirect_to(person_path(person))
        expect(flash[:alert]).to include('Cannot take medicine')
      end
    end

    context 'when minimum hours between doses not met' do
      before do
        MedicationTake.create!(
          prescription: prescription,
          taken_at: 1.hour.ago,
          amount_ml: 10
        )
      end

      it 'does not create a medication take' do
        expect do
          post take_medicine_person_prescription_path(person, prescription)
        end.not_to change(MedicationTake, :count)

        expect(response).to redirect_to(person_path(person))
        expect(flash[:alert]).to include('Cannot take medicine')
      end
    end
  end

  describe 'POST /people/:person_id/person_medicines/:id/take_medicine' do
    let(:person_medicine) do
      PersonMedicine.create!(
        person: person,
        medicine: medicine,
        max_daily_doses: 2,
        min_hours_between_doses: 4
      )
    end

    context 'when timing restrictions allow taking medicine' do
      it 'creates a medication take' do
        expect do
          post take_medicine_person_person_medicine_path(person, person_medicine)
        end.to change(MedicationTake, :count).by(1)

        expect(response).to redirect_to(person_path(person))
        expect(flash[:notice]).to include('Medicine taken')
      end
    end

    context 'when max daily doses reached' do
      before do
        2.times do
          MedicationTake.create!(
            person_medicine: person_medicine,
            taken_at: Time.current,
            amount_ml: 10
          )
        end
      end

      it 'does not create a medication take' do
        expect do
          post take_medicine_person_person_medicine_path(person, person_medicine)
        end.not_to change(MedicationTake, :count)

        expect(response).to redirect_to(person_path(person))
        expect(flash[:alert]).to include('Cannot take medicine')
      end
    end

    context 'when minimum hours between doses not met' do
      before do
        MedicationTake.create!(
          person_medicine: person_medicine,
          taken_at: 1.hour.ago,
          amount_ml: 10
        )
      end

      it 'does not create a medication take' do
        expect do
          post take_medicine_person_person_medicine_path(person, person_medicine)
        end.not_to change(MedicationTake, :count)

        expect(response).to redirect_to(person_path(person))
        expect(flash[:alert]).to include('Cannot take medicine')
      end
    end
  end
end
