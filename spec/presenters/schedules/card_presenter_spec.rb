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
      expect(presenter.dose_description).to eq('500mg • Twice daily')
    end

    it 'truncates fractional dose amounts to integer' do
      allow(schedule).to receive(:dose_amount).and_return(250.7)
      expect(presenter.dose_description).to eq('250mg • Twice daily')
    end

    it 'works for non-mg units' do
      allow(schedule).to receive_messages(dose_amount: 5, dose_unit: 'ml', frequency: 'Once daily')

      expect(presenter.dose_description).to eq('5ml • Once daily')
    end
  end

  describe '#dose_description (dose_text detail)' do
    it 'uses to_i so a float dose like 2.9 shows as 2 (truncates, not rounds)' do
      allow(schedule).to receive(:dose_amount).and_return(2.9)
      # to_i truncates toward zero — 2.9.to_i == 2
      expect(presenter.dose_description).to start_with('2mg')
    end

    it 'handles zero dose amount' do
      allow(schedule).to receive(:dose_amount).and_return(0)
      expect(presenter.dose_description).to start_with('0mg')
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
