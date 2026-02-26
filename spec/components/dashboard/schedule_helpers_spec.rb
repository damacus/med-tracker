# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::ScheduleHelpers do
  let(:test_class) do
    Class.new do
      include Components::Dashboard::ScheduleHelpers

      attr_reader :schedule, :current_user

      def initialize(schedule:, current_user: nil)
        @schedule = schedule
        @current_user = current_user
      end
    end
  end

  let(:schedule) do
    person = create(:person)
    medication = create(:medication)
    dosage = Dosage.new(medication: medication, amount: 500.0, unit: 'mg')
    Schedule.new(
      person: person,
      medication: medication,
      dosage: dosage,
      end_date: Date.new(2024, 12, 31),
      frequency: 'Twice daily'
    )
  end

  let(:user) { instance_double(User, email_address: 'test@example.com', role: :administrator) }
  let(:instance) { test_class.new(schedule: schedule, current_user: user) }

  describe '#format_dosage' do
    context 'when dosage has amount and unit' do
      it 'formats integer amounts without decimal' do
        expect(instance.format_dosage).to eq('500 mg')
      end

      it 'formats decimal amounts with decimal' do
        person = create(:person)
        medication = create(:medication)
        dosage = Dosage.new(medication: medication, amount: 2.5, unit: 'ml')
        custom_schedule = Schedule.new(person: person, medication: medication, dosage: dosage)
        custom_instance = test_class.new(schedule: custom_schedule, current_user: user)

        expect(custom_instance.format_dosage).to eq('2.5 ml')
      end
    end

    context 'when dosage is missing amount' do
      it 'returns em dash' do
        person = create(:person)
        medication = create(:medication)
        dosage = Dosage.new(medication: medication, amount: nil, unit: 'mg')
        custom_schedule = Schedule.new(person: person, medication: medication, dosage: dosage)
        custom_instance = test_class.new(schedule: custom_schedule, current_user: user)

        expect(custom_instance.format_dosage).to eq('—')
      end
    end

    context 'when dosage is missing unit' do
      it 'returns em dash' do
        person = create(:person)
        medication = create(:medication)
        dosage = Dosage.new(medication: medication, amount: 500.0, unit: nil)
        custom_schedule = Schedule.new(person: person, medication: medication, dosage: dosage)
        custom_instance = test_class.new(schedule: custom_schedule, current_user: user)

        expect(custom_instance.format_dosage).to eq('—')
      end
    end

    context 'when dosage is nil' do
      it 'returns em dash' do
        person = create(:person)
        medication = create(:medication)
        custom_schedule = Schedule.new(person: person, medication: medication, dosage: nil)
        custom_instance = test_class.new(schedule: custom_schedule, current_user: user)

        expect(custom_instance.format_dosage).to eq('—')
      end
    end
  end

  describe '#format_quantity' do
    context 'when medication has remaining supply' do
      it 'returns the remaining supply as a string' do
        schedule.medication.current_supply = 44
        expect(instance.format_quantity).to eq('44')
      end
    end

    context 'when medication remaining supply is nil' do
      it 'returns em dash' do
        schedule.medication.current_supply = nil
        expect(instance.format_quantity).to eq('—')
      end
    end

    context 'when medication is nil' do
      it 'returns em dash' do
        person = create(:person)
        custom_schedule = Schedule.new(person: person, medication: nil, dosage: nil)
        custom_instance = test_class.new(schedule: custom_schedule, current_user: user)

        expect(custom_instance.format_quantity).to eq('—')
      end
    end
  end

  describe '#format_end_date' do
    context 'when schedule has end_date' do
      it 'formats the date' do
        schedule.end_date = Date.new(2024, 12, 31)
        expect(instance.format_end_date).to eq('Dec 31, 2024')
      end
    end

    context 'when schedule has no end_date' do
      it 'returns em dash' do
        schedule.end_date = nil
        expect(instance.format_end_date).to eq('—')
      end
    end
  end

  describe '#can_delete?' do
    context 'when current_user is nil' do
      let(:instance) { test_class.new(schedule: schedule, current_user: nil) }

      it 'returns false' do
        expect(instance.can_delete?).to be false
      end
    end

    context 'when current_user is present' do
      it 'delegates to SchedulePolicy' do
        policy_double = instance_double(SchedulePolicy, destroy?: true)
        allow(SchedulePolicy).to receive(:new).with(user, schedule).and_return(policy_double)

        expect(instance.can_delete?).to be true
        expect(SchedulePolicy).to have_received(:new).with(user, schedule)
      end

      it 'returns false when policy denies destroy' do
        policy_double = instance_double(SchedulePolicy, destroy?: false)
        allow(SchedulePolicy).to receive(:new).with(user, schedule).and_return(policy_double)

        expect(instance.can_delete?).to be false
      end
    end
  end
end
