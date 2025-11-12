# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PersonMedicine do
  describe '#can_take_now?' do
    context 'without timing restrictions' do
      let(:person_medicine) { create(:person_medicine) }

      it 'returns true' do
        expect(person_medicine.can_take_now?).to be true
      end
    end

    context 'with max_daily_doses restriction' do
      let(:person_medicine) { create(:person_medicine, max_daily_doses: 1) }

      it 'returns false when max doses reached' do
        create(:medication_take, :for_person_medicine, :today, person_medicine: person_medicine)
        expect(person_medicine.can_take_now?).to be false
      end

      it 'returns true when under max doses' do
        expect(person_medicine.can_take_now?).to be true
      end
    end

    context 'with min_hours_between_doses restriction' do
      let(:person_medicine) { create(:person_medicine, min_hours_between_doses: 4) }

      it 'returns false when minimum hours not passed' do
        create(:medication_take, :for_person_medicine, :recent, person_medicine: person_medicine)
        expect(person_medicine.can_take_now?).to be false
      end

      it 'returns true when minimum hours passed' do
        expect(person_medicine.can_take_now?).to be true
      end
    end

    context 'with both restrictions' do
      let(:person_medicine) { create(:person_medicine, :with_both_restrictions) }

      it 'returns false when either restriction violated' do
        create(:medication_take, :for_person_medicine, :recent, person_medicine: person_medicine)
        expect(person_medicine.can_take_now?).to be false
      end
    end
  end

  describe '#next_available_time' do
    context 'without timing restrictions' do
      let(:person_medicine) { create(:person_medicine) }

      it 'returns nil' do
        expect(person_medicine.next_available_time).to be_nil
      end
    end

    context 'when can take now' do
      let(:person_medicine) { create(:person_medicine, :with_both_restrictions) }

      it 'returns current time' do
        expect(person_medicine.next_available_time).to be_within(1.second).of(Time.current)
      end
    end

    context 'with min_hours_between_doses restriction' do
      let(:person_medicine) { create(:person_medicine, min_hours_between_doses: 4) }
      let!(:last_take) { create(:medication_take, :for_person_medicine, :recent, person_medicine: person_medicine) }

      it 'returns time when minimum hours will be satisfied' do
        expected_time = last_take.taken_at + person_medicine.min_hours_between_doses.hours

        expect(person_medicine.next_available_time).to be_within(1.second).of(expected_time)
      end
    end

    context 'with max_daily_doses restriction' do
      let(:person_medicine) { create(:person_medicine, max_daily_doses: 1) }

      it 'returns start of next day when max doses reached' do
        create(:medication_take, :for_person_medicine, :today, person_medicine: person_medicine)
        expected_time = Time.current.end_of_day + 1.second

        expect(person_medicine.next_available_time).to be_within(1.second).of(expected_time)
      end
    end

    context 'with both restrictions' do
      let(:person_medicine) { create(:person_medicine, :with_both_restrictions) }

      it 'returns the earliest time when restrictions are satisfied' do
        take = create(:medication_take, :for_person_medicine, :recent, person_medicine: person_medicine)
        expected_time = take.taken_at + person_medicine.min_hours_between_doses.hours

        expect(person_medicine.next_available_time).to be_within(1.second).of(expected_time)
      end
    end
  end

  describe '#time_until_next_dose' do
    context 'when can take now' do
      let(:person_medicine) { create(:person_medicine, :with_both_restrictions) }

      it 'returns nil' do
        expect(person_medicine.time_until_next_dose).to be_nil
      end
    end

    context 'when restricted' do
      let(:person_medicine) { create(:person_medicine, min_hours_between_doses: 4) }

      it 'returns seconds until next available time' do
        create(:medication_take, :for_person_medicine, :recent, person_medicine: person_medicine)
        next_time = person_medicine.next_available_time
        expected_seconds = (next_time - Time.current).to_i

        expect(person_medicine.time_until_next_dose).to be_within(2).of(expected_seconds)
      end
    end
  end

  describe '#countdown_display' do
    context 'when can take now' do
      let(:person_medicine) { create(:person_medicine, :with_both_restrictions) }

      it 'returns nil' do
        expect(person_medicine.countdown_display).to be_nil
      end
    end

    context 'with hours remaining' do
      let(:person_medicine) { create(:person_medicine, min_hours_between_doses: 4) }

      it 'returns formatted time string' do
        create(:medication_take, :for_person_medicine, :recent, person_medicine: person_medicine)
        expect(person_medicine.countdown_display).to match(/\d+h \d+m/)
      end
    end

    context 'with only minutes remaining' do
      let(:person_medicine) { create(:person_medicine, min_hours_between_doses: 4) }

      before do
        taken_at_time = person_medicine.min_hours_between_doses.hours - 30.minutes
        create(:medication_take, :for_person_medicine, person_medicine: person_medicine,
                                                       taken_at: taken_at_time.ago)
      end

      it 'returns formatted time string with minutes only' do
        expect(person_medicine.countdown_display).to match(/\d+m/)
      end
    end

    context 'with less than a minute remaining' do
      let(:person_medicine) { create(:person_medicine, min_hours_between_doses: 4) }

      before do
        taken_at_time = person_medicine.min_hours_between_doses.hours - 30.seconds
        create(:medication_take, :for_person_medicine, person_medicine: person_medicine,
                                                       taken_at: taken_at_time.ago)
      end

      it 'returns "less than 1 minute"' do
        expect(person_medicine.countdown_display).to eq('less than 1 minute')
      end
    end
  end
end
