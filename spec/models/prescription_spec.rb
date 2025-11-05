# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Prescription do
  fixtures :prescriptions, :people, :medicines, :dosages

  describe 'active flag' do
    let(:prescription) { prescriptions(:john_paracetamol) }

    it 'is active by default' do
      new_prescription = described_class.new(
        person: people(:john),
        medicine: medicines(:paracetamol),
        dosage: dosages(:paracetamol_adult),
        start_date: Time.zone.today,
        end_date: Time.zone.today + 30.days
      )
      new_prescription.save
      expect(new_prescription.active).to be true
    end

    it 'can be set to inactive' do
      prescription.update(active: false)
      expect(prescription.active).to be false
    end

    it 'can be reactivated' do
      prescription.update(active: false)
      prescription.update(active: true)
      expect(prescription.active).to be true
    end
  end

  describe 'validations' do
    subject(:prescription) do
      described_class.new(
        person: person,
        medicine: medicine,
        dosage: dosage,
        start_date: Time.zone.today,
        end_date: Time.zone.today + 30.days
      )
    end

    let(:person) do
      Person.create!(
        name: 'Jane Doe',
        email: 'jane@example.com',
        date_of_birth: Date.new(1990, 1, 1)
      )
    end
    let(:medicine) do
      Medicine.create!(
        name: 'Lisinopril',
        current_supply: 50,
        stock: 50,
        reorder_threshold: 10
      )
    end
    let(:dosage) { Dosage.create!(medicine: medicine, amount: 10, unit: 'mg', frequency: 'daily') }

    it { is_expected.to validate_presence_of(:start_date) }
    it { is_expected.to validate_presence_of(:end_date) }

    it 'is invalid if end_date is before start_date' do
      prescription.end_date = prescription.start_date - 1.day
      expect(prescription).not_to be_valid
    end
  end

  describe 'associations' do
    it { is_expected.to belong_to(:person) }
    it { is_expected.to belong_to(:dosage) }
    it { is_expected.to have_many(:medication_takes).dependent(:destroy) }

    it 'has take_medicines as an alias for medication_takes' do
      prescription = prescriptions(:john_paracetamol)
      expect(prescription).to respond_to(:take_medicines)
      expect(prescription.take_medicines).to eq(prescription.medication_takes)
    end
  end

  describe 'timing restrictions' do
    let(:prescription) { prescriptions(:john_paracetamol) }

    describe '#timing_restrictions?' do
      it 'returns true when max_daily_doses is present' do
        prescription.update(max_daily_doses: 4, min_hours_between_doses: nil)
        expect(prescription.timing_restrictions?).to be true
      end

      it 'returns true when min_hours_between_doses is present' do
        prescription.update(max_daily_doses: nil, min_hours_between_doses: 6)
        expect(prescription.timing_restrictions?).to be true
      end

      it 'returns true when both restrictions are present' do
        prescription.update(max_daily_doses: 4, min_hours_between_doses: 6)
        expect(prescription.timing_restrictions?).to be true
      end

      it 'returns false when no restrictions are present' do
        prescription.update(max_daily_doses: nil, min_hours_between_doses: nil)
        expect(prescription.timing_restrictions?).to be false
      end
    end

    describe '#can_take_now?' do
      context 'when no timing restrictions are set' do
        before do
          prescription.update(max_daily_doses: nil, min_hours_between_doses: nil)
        end

        it 'returns true' do
          expect(prescription.can_take_now?).to be true
        end
      end

      context 'with max_daily_doses restriction' do
        before do
          prescription.update(max_daily_doses: 2, min_hours_between_doses: nil)
        end

        it 'returns true when under max doses for today' do
          prescription.medication_takes.create!(taken_at: 8.hours.ago)
          expect(prescription.can_take_now?).to be true
        end

        it 'returns false when max doses reached for today' do
          prescription.medication_takes.create!(taken_at: 8.hours.ago)
          prescription.medication_takes.create!(taken_at: 4.hours.ago)
          expect(prescription.can_take_now?).to be false
        end

        it 'returns true the next day after max doses' do
          prescription.medication_takes.create!(taken_at: 1.day.ago.beginning_of_day + 8.hours)
          prescription.medication_takes.create!(taken_at: 1.day.ago.beginning_of_day + 12.hours)
          expect(prescription.can_take_now?).to be true
        end
      end

      context 'with min_hours_between_doses restriction' do
        before do
          prescription.update(max_daily_doses: nil, min_hours_between_doses: 4)
        end

        it 'returns true when no previous doses' do
          expect(prescription.can_take_now?).to be true
        end

        it 'returns true when enough time has passed' do
          prescription.medication_takes.create!(taken_at: 5.hours.ago)
          expect(prescription.can_take_now?).to be true
        end

        it 'returns false when not enough time has passed' do
          prescription.medication_takes.create!(taken_at: 2.hours.ago)
          expect(prescription.can_take_now?).to be false
        end
      end

      context 'with both restrictions' do
        before do
          prescription.update(max_daily_doses: 3, min_hours_between_doses: 4)
        end

        it 'returns false when max doses reached even if hours passed' do
          prescription.medication_takes.create!(taken_at: 12.hours.ago)
          prescription.medication_takes.create!(taken_at: 8.hours.ago)
          prescription.medication_takes.create!(taken_at: 5.hours.ago)
          expect(prescription.can_take_now?).to be false
        end

        it 'returns false when not enough hours passed even if under max doses' do
          prescription.medication_takes.create!(taken_at: 2.hours.ago)
          expect(prescription.can_take_now?).to be false
        end

        it 'returns true when both restrictions are satisfied' do
          prescription.medication_takes.create!(taken_at: 8.hours.ago)
          expect(prescription.can_take_now?).to be true
        end
      end
    end

    describe '#next_available_time' do
      context 'when no timing restrictions are set' do
        before do
          prescription.update(max_daily_doses: nil, min_hours_between_doses: nil)
        end

        it 'returns nil' do
          expect(prescription.next_available_time).to be_nil
        end
      end

      context 'when can take now' do
        before do
          prescription.update(max_daily_doses: 4, min_hours_between_doses: 4)
        end

        it 'returns current time' do
          expect(prescription.next_available_time).to be_within(1.second).of(Time.current)
        end
      end

      context 'when blocked by min_hours_between_doses' do
        before do
          prescription.update(max_daily_doses: nil, min_hours_between_doses: 4)
          prescription.medication_takes.create!(taken_at: 2.hours.ago)
        end

        it 'returns time when min hours will be satisfied' do
          expected_time = 2.hours.from_now
          expect(prescription.next_available_time).to be_within(1.minute).of(expected_time)
        end
      end

      context 'when blocked by max_daily_doses' do
        before do
          prescription.update(max_daily_doses: 2, min_hours_between_doses: nil)
          prescription.medication_takes.create!(taken_at: 8.hours.ago)
          prescription.medication_takes.create!(taken_at: 4.hours.ago)
        end

        it 'returns start of next day' do
          expected_time = Time.current.end_of_day + 1.second
          expect(prescription.next_available_time).to be_within(1.minute).of(expected_time)
        end
      end

      context 'when blocked by both restrictions' do
        before do
          prescription.update(max_daily_doses: 3, min_hours_between_doses: 6)
          prescription.medication_takes.create!(taken_at: 10.hours.ago)
          prescription.medication_takes.create!(taken_at: 8.hours.ago)
          prescription.medication_takes.create!(taken_at: 4.hours.ago)
        end

        it 'returns the earliest available time' do
          # max_daily_doses is blocking (3 doses taken), so next day
          expected_time = Time.current.end_of_day + 1.second
          expect(prescription.next_available_time).to be_within(1.minute).of(expected_time)
        end
      end
    end
  end
end
