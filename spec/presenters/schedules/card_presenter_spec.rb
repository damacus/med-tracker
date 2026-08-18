# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Schedules::CardPresenter do
  subject(:presenter) { described_class.new(schedule: schedule, current_user: current_user, person: person) }

  let(:schedule) do
    instance_double(Schedule,
                    dose_amount: 500,
                    dose_unit: 'mg',
                    frequency: 'Twice daily')
  end
  let(:person) { instance_double(Person) }
  let(:current_user) { instance_double(User) }

  describe '#dose_description' do
    it 'combines dose text and frequency with a bullet separator' do
      expect(presenter.dose_description).to eq('500 mg • Twice daily')
    end

    it 'preserves fractional tablet doses' do
      allow(schedule).to receive_messages(dose_amount: 0.5, dose_unit: 'tablet')

      expect(presenter.dose_description).to eq('0.5 tablets • Twice daily')
    end

    it 'keeps one tablet singular' do
      allow(schedule).to receive_messages(dose_amount: 1.0, dose_unit: 'tablet', frequency: 'Once daily')

      expect(presenter.dose_description).to eq('1 tablet • Once daily')
    end

    it 'formats other measurement units' do
      allow(schedule).to receive_messages(dose_amount: 5, dose_unit: 'ml', frequency: 'Once daily')

      expect(presenter.dose_description).to eq('5 ml • Once daily')
    end

    it 'handles zero dose amount' do
      allow(schedule).to receive(:dose_amount).and_return(0)

      expect(presenter.dose_description).to eq('0 mg • Twice daily')
    end
  end

  describe 'accessors' do
    it 'exposes schedule' do
      expect(presenter.schedule).to eq(schedule)
    end

    it 'exposes current_user' do
      expect(presenter.current_user).to eq(current_user)
    end

    it 'exposes person' do
      expect(presenter.person).to eq(person)
    end
  end

  # own_dose? is private but tested here to ensure mutation coverage
  describe 'private #own_dose?' do
    def own_dose?(presenter)
      presenter.send(:own_dose?)
    end

    context 'when current_user is nil' do
      subject(:presenter) { described_class.new(schedule: schedule, current_user: nil, person: person) }

      it 'returns true (anyone can view dose for anonymous context)' do
        expect(own_dose?(presenter)).to be(true)
      end
    end

    context 'when current_user.person matches person' do
      subject(:presenter) do
        described_class.new(schedule: schedule, current_user: matching_user, person: matching_person)
      end

      let(:matching_person) { instance_double(Person) }
      let(:matching_user)   { instance_double(User, person: matching_person) }

      it 'returns true' do
        expect(own_dose?(presenter)).to be(true)
      end
    end

    context 'when current_user.person does not match person' do
      let(:other_person)  { instance_double(Person) }
      let(:current_user)  { instance_double(User, person: other_person) }

      it 'returns false' do
        expect(own_dose?(presenter)).to be(false)
      end
    end
  end
end
