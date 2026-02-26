# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PersonMedication do
  describe '#can_take_now?' do
    context 'without timing restrictions' do
      let(:person_medication) { create(:person_medication) }

      it 'returns true' do
        expect(person_medication.can_take_now?).to be true
      end
    end

    context 'with max_daily_doses restriction' do
      let(:person_medication) { create(:person_medication, max_daily_doses: 1) }

      it 'returns false when max doses reached' do
        create(:medication_take, :for_person_medication, :today, person_medication: person_medication)
        expect(person_medication.can_take_now?).to be false
      end

      it 'returns true when under max doses' do
        expect(person_medication.can_take_now?).to be true
      end
    end

    context 'with min_hours_between_doses restriction' do
      let(:person_medication) { create(:person_medication, min_hours_between_doses: 4) }

      it 'returns false when minimum hours not passed' do
        create(:medication_take, :for_person_medication, :recent, person_medication: person_medication)
        expect(person_medication.can_take_now?).to be false
      end

      it 'returns true when minimum hours passed' do
        expect(person_medication.can_take_now?).to be true
      end
    end

    context 'with both restrictions' do
      let(:person_medication) { create(:person_medication, :with_both_restrictions) }

      it 'returns false when either restriction violated' do
        create(:medication_take, :for_person_medication, :recent, person_medication: person_medication)
        expect(person_medication.can_take_now?).to be false
      end
    end
  end

  describe '#next_available_time' do
    context 'without timing restrictions' do
      let(:person_medication) { create(:person_medication) }

      it 'returns nil' do
        expect(person_medication.next_available_time).to be_nil
      end
    end

    context 'when can take now' do
      let(:person_medication) { create(:person_medication, :with_both_restrictions) }

      it 'returns current time' do
        expect(person_medication.next_available_time).to be_within(1.second).of(Time.current)
      end
    end

    context 'with min_hours_between_doses restriction' do
      let(:person_medication) { create(:person_medication, min_hours_between_doses: 4) }
      let!(:last_take) do
        create(:medication_take, :for_person_medication, :recent, person_medication: person_medication)
      end

      it 'returns time when minimum hours will be satisfied' do
        expected_time = last_take.taken_at + person_medication.min_hours_between_doses.hours

        expect(person_medication.next_available_time).to be_within(1.second).of(expected_time)
      end
    end

    context 'with max_daily_doses restriction' do
      let(:person_medication) { create(:person_medication, max_daily_doses: 1) }

      it 'returns start of next day when max doses reached' do
        create(:medication_take, :for_person_medication, :today, person_medication: person_medication)
        expected_time = Time.current.end_of_day + 1.second

        expect(person_medication.next_available_time).to be_within(1.second).of(expected_time)
      end
    end

    context 'with both restrictions' do
      let(:person_medication) { create(:person_medication, :with_both_restrictions) }

      it 'returns the earliest time when restrictions are satisfied' do
        take = create(:medication_take, :for_person_medication, :recent, person_medication: person_medication)
        expected_time = take.taken_at + person_medication.min_hours_between_doses.hours

        expect(person_medication.next_available_time).to be_within(1.second).of(expected_time)
      end
    end
  end

  describe '#time_until_next_dose' do
    context 'when can take now' do
      let(:person_medication) { create(:person_medication, :with_both_restrictions) }

      it 'returns nil' do
        expect(person_medication.time_until_next_dose).to be_nil
      end
    end

    context 'when restricted' do
      let(:person_medication) { create(:person_medication, min_hours_between_doses: 4) }

      it 'returns seconds until next available time' do
        create(:medication_take, :for_person_medication, :recent, person_medication: person_medication)
        next_time = person_medication.next_available_time
        expected_seconds = (next_time - Time.current).to_i

        expect(person_medication.time_until_next_dose).to be_within(2).of(expected_seconds)
      end
    end
  end

  describe '#can_administer?' do
    context 'when can take now and medication in stock' do
      let(:person_medication) { create(:person_medication) }

      it 'returns true' do
        expect(person_medication.can_administer?).to be true
      end
    end

    context 'when on cooldown' do
      let(:person_medication) { create(:person_medication, min_hours_between_doses: 4) }

      it 'returns false' do
        create(:medication_take, :for_person_medication, :recent, person_medication: person_medication)
        expect(person_medication.can_administer?).to be false
      end
    end

    context 'when medication is out of stock' do
      let(:medication) { create(:medication, current_supply: 0) }
      let(:person_medication) { create(:person_medication, medication: medication) }

      it 'returns false' do
        expect(person_medication.can_administer?).to be false
      end
    end

    context 'when both on cooldown and out of stock' do
      let(:medication) { create(:medication, current_supply: 0) }
      let(:person_medication) { create(:person_medication, medication: medication, min_hours_between_doses: 4) }

      it 'returns false' do
        create(:medication_take, :for_person_medication, :recent, person_medication: person_medication)
        expect(person_medication.can_administer?).to be false
      end
    end

    context 'when medication current_supply is nil (untracked)' do
      let(:medication) { create(:medication, current_supply: nil) }
      let(:person_medication) { create(:person_medication, medication: medication) }

      it 'returns true (nil current_supply means untracked)' do
        expect(person_medication.can_administer?).to be true
      end
    end
  end

  describe '#administration_blocked_reason' do
    context 'when can administer' do
      let(:person_medication) { create(:person_medication) }

      it 'returns nil' do
        expect(person_medication.administration_blocked_reason).to be_nil
      end
    end

    context 'when on cooldown' do
      let(:person_medication) { create(:person_medication, min_hours_between_doses: 4) }

      it 'returns :cooldown' do
        create(:medication_take, :for_person_medication, :recent, person_medication: person_medication)
        expect(person_medication.administration_blocked_reason).to eq(:cooldown)
      end
    end

    context 'when out of stock' do
      let(:medication) { create(:medication, current_supply: 0) }
      let(:person_medication) { create(:person_medication, medication: medication) }

      it 'returns :out_of_stock' do
        expect(person_medication.administration_blocked_reason).to eq(:out_of_stock)
      end
    end

    context 'when both on cooldown and out of stock' do
      let(:medication) { create(:medication, current_supply: 0) }
      let(:person_medication) { create(:person_medication, medication: medication, min_hours_between_doses: 4) }

      it 'returns :out_of_stock (stock takes priority)' do
        create(:medication_take, :for_person_medication, :recent, person_medication: person_medication)
        expect(person_medication.administration_blocked_reason).to eq(:out_of_stock)
      end
    end
  end

  describe '#countdown_display' do
    context 'when can take now' do
      let(:person_medication) { create(:person_medication, :with_both_restrictions) }

      it 'returns nil' do
        expect(person_medication.countdown_display).to be_nil
      end
    end

    context 'with hours remaining' do
      let(:person_medication) { create(:person_medication, min_hours_between_doses: 4) }

      it 'returns formatted time string' do
        create(:medication_take, :for_person_medication, :recent, person_medication: person_medication)
        expect(person_medication.countdown_display).to match(/\d+h \d+m/)
      end
    end

    context 'with only minutes remaining' do
      let(:person_medication) { create(:person_medication, min_hours_between_doses: 4) }

      before do
        taken_at_time = person_medication.min_hours_between_doses.hours - 30.minutes
        create(:medication_take, :for_person_medication, person_medication: person_medication,
                                                         taken_at: taken_at_time.ago)
      end

      it 'returns formatted time string with minutes only' do
        expect(person_medication.countdown_display).to match(/\d+m/)
      end
    end

    context 'with less than a minute remaining' do
      let(:person_medication) { create(:person_medication, min_hours_between_doses: 4) }

      before do
        taken_at_time = person_medication.min_hours_between_doses.hours - 30.seconds
        create(:medication_take, :for_person_medication, person_medication: person_medication,
                                                         taken_at: taken_at_time.ago)
      end

      it 'returns "less than 1 minute"' do
        expect(person_medication.countdown_display).to eq('less than 1 minute')
      end
    end
  end
end
