# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NotificationPreference, type: :model do
  subject(:notification_preference) { build(:notification_preference) }

  describe 'associations' do
    it { is_expected.to belong_to(:person) }
  end

  describe 'constants' do
    it 'defines PERIODS' do
      expect(described_class::PERIODS).to eq(%i[morning afternoon evening night])
    end
  end

  describe '#time_for_period' do
    let(:preference) do
      build(:notification_preference,
            morning_time: '08:00:00',
            afternoon_time: '14:00:00',
            evening_time: '18:00:00',
            night_time: '22:00:00')
    end

    context 'with valid periods' do
      it 'returns the correct time for morning' do
        expect(preference.time_for_period(:morning).to_s(:time)).to eq('08:00')
      end

      it 'returns the correct time for afternoon' do
        expect(preference.time_for_period(:afternoon).to_s(:time)).to eq('14:00')
      end

      it 'returns the correct time for evening' do
        expect(preference.time_for_period(:evening).to_s(:time)).to eq('18:00')
      end

      it 'returns the correct time for night' do
        expect(preference.time_for_period(:night).to_s(:time)).to eq('22:00')
      end
    end

    context 'with invalid periods' do
      it 'raises a NoMethodError' do
        expect { preference.time_for_period(:invalid_period) }.to raise_error(NoMethodError)
      end
    end
  end
end
