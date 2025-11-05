# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PersonMedicine do
  fixtures :person_medicines, :people, :medicines

  describe 'validations' do
    it 'validates uniqueness of person_id scoped to medicine_id' do
      person_medicine = person_medicines(:john_vitamin_d)
      duplicate = described_class.new(
        person: person_medicine.person,
        medicine: person_medicine.medicine
      )
      expect(duplicate).not_to be_valid
      expect(duplicate.errors[:person_id]).to include('has already been taken')
    end
  end

  describe 'associations' do
    it { is_expected.to belong_to(:person) }
    it { is_expected.to belong_to(:medicine) }
    it { is_expected.to have_many(:medication_takes).dependent(:destroy) }
  end

  describe 'timing restrictions' do
    let(:person_medicine) { person_medicines(:john_vitamin_d) }

    describe '#timing_restrictions?' do
      it 'returns true when max_daily_doses is present' do
        person_medicine.update(max_daily_doses: 4, min_hours_between_doses: nil)
        expect(person_medicine.timing_restrictions?).to be true
      end

      it 'returns true when min_hours_between_doses is present' do
        person_medicine.update(max_daily_doses: nil, min_hours_between_doses: 6)
        expect(person_medicine.timing_restrictions?).to be true
      end

      it 'returns true when both restrictions are present' do
        person_medicine.update(max_daily_doses: 4, min_hours_between_doses: 6)
        expect(person_medicine.timing_restrictions?).to be true
      end

      it 'returns false when no restrictions are present' do
        person_medicine.update(max_daily_doses: nil, min_hours_between_doses: nil)
        expect(person_medicine.timing_restrictions?).to be false
      end
    end

    describe '#cycle_period' do
      it 'returns 1 day for person medicines' do
        expect(person_medicine.cycle_period).to eq(1.day)
      end
    end

    describe '#can_take_now?' do
      context 'when no timing restrictions are set' do
        before do
          person_medicine.update(max_daily_doses: nil, min_hours_between_doses: nil)
        end

        it 'returns true' do
          expect(person_medicine.can_take_now?).to be true
        end
      end

      context 'with max_daily_doses restriction' do
        before do
          person_medicine.update(max_daily_doses: 2, min_hours_between_doses: nil)
        end

        it 'returns true when under max doses for today' do
          person_medicine.medication_takes.create!(taken_at: 8.hours.ago)
          expect(person_medicine.can_take_now?).to be true
        end

        it 'returns false when max doses reached for today' do
          person_medicine.medication_takes.create!(taken_at: 8.hours.ago)
          person_medicine.medication_takes.create!(taken_at: 4.hours.ago)
          expect(person_medicine.can_take_now?).to be false
        end

        it 'returns true the next day after max doses' do
          person_medicine.medication_takes.create!(taken_at: 1.day.ago.beginning_of_day + 8.hours)
          person_medicine.medication_takes.create!(taken_at: 1.day.ago.beginning_of_day + 12.hours)
          expect(person_medicine.can_take_now?).to be true
        end
      end

      context 'with min_hours_between_doses restriction' do
        before do
          person_medicine.update(max_daily_doses: nil, min_hours_between_doses: 4)
        end

        it 'returns true when no previous doses' do
          expect(person_medicine.can_take_now?).to be true
        end

        it 'returns true when enough time has passed' do
          person_medicine.medication_takes.create!(taken_at: 5.hours.ago)
          expect(person_medicine.can_take_now?).to be true
        end

        it 'returns false when not enough time has passed' do
          person_medicine.medication_takes.create!(taken_at: 2.hours.ago)
          expect(person_medicine.can_take_now?).to be false
        end
      end

      context 'with both restrictions' do
        before do
          person_medicine.update(max_daily_doses: 3, min_hours_between_doses: 4)
        end

        it 'returns false when max doses reached even if hours passed' do
          person_medicine.medication_takes.create!(taken_at: 12.hours.ago)
          person_medicine.medication_takes.create!(taken_at: 8.hours.ago)
          person_medicine.medication_takes.create!(taken_at: 5.hours.ago)
          expect(person_medicine.can_take_now?).to be false
        end

        it 'returns false when not enough hours passed even if under max doses' do
          person_medicine.medication_takes.create!(taken_at: 2.hours.ago)
          expect(person_medicine.can_take_now?).to be false
        end

        it 'returns true when both restrictions are satisfied' do
          person_medicine.medication_takes.create!(taken_at: 8.hours.ago)
          expect(person_medicine.can_take_now?).to be true
        end
      end
    end

    describe '#next_available_time' do
      context 'when no timing restrictions are set' do
        before do
          person_medicine.update(max_daily_doses: nil, min_hours_between_doses: nil)
        end

        it 'returns nil' do
          expect(person_medicine.next_available_time).to be_nil
        end
      end

      context 'when can take now' do
        before do
          person_medicine.update(max_daily_doses: 4, min_hours_between_doses: 4)
        end

        it 'returns current time' do
          expect(person_medicine.next_available_time).to be_within(1.second).of(Time.current)
        end
      end

      context 'when blocked by min_hours_between_doses' do
        before do
          person_medicine.update(max_daily_doses: nil, min_hours_between_doses: 4)
          person_medicine.medication_takes.create!(taken_at: 2.hours.ago)
        end

        it 'returns time when min hours will be satisfied' do
          expected_time = 2.hours.from_now
          expect(person_medicine.next_available_time).to be_within(1.minute).of(expected_time)
        end
      end

      context 'when blocked by max_daily_doses' do
        before do
          person_medicine.update(max_daily_doses: 2, min_hours_between_doses: nil)
          person_medicine.medication_takes.create!(taken_at: 8.hours.ago)
          person_medicine.medication_takes.create!(taken_at: 4.hours.ago)
        end

        it 'returns start of next day' do
          expected_time = Time.current.end_of_day + 1.second
          expect(person_medicine.next_available_time).to be_within(1.minute).of(expected_time)
        end
      end

      context 'when blocked by both restrictions' do
        before do
          person_medicine.update(max_daily_doses: 3, min_hours_between_doses: 6)
          person_medicine.medication_takes.create!(taken_at: 10.hours.ago)
          person_medicine.medication_takes.create!(taken_at: 8.hours.ago)
          person_medicine.medication_takes.create!(taken_at: 4.hours.ago)
        end

        it 'returns the earliest available time' do
          # max_daily_doses is blocking (3 doses taken), so next day
          expected_time = Time.current.end_of_day + 1.second
          expect(person_medicine.next_available_time).to be_within(1.minute).of(expected_time)
        end
      end
    end
  end
end
