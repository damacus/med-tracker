# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NotificationPreference do
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
      it 'returns the morning_time attribute' do
        expect(preference.time_for_period(:morning)).to eq(preference.morning_time)
      end

      it 'returns the afternoon_time attribute' do
        expect(preference.time_for_period(:afternoon)).to eq(preference.afternoon_time)
      end

      it 'returns the evening_time attribute' do
        expect(preference.time_for_period(:evening)).to eq(preference.evening_time)
      end

      it 'returns the night_time attribute' do
        expect(preference.time_for_period(:night)).to eq(preference.night_time)
      end
    end

    context 'with invalid periods' do
      it 'raises a NoMethodError' do
        expect { preference.time_for_period(:invalid_period) }.to raise_error(NoMethodError)
      end
    end
  end
end
