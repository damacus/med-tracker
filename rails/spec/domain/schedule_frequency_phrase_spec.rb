# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ScheduleFrequencyPhrase do
  describe '#to_s' do
    it 'describes weekly limits in plain English' do
      phrase = described_class.new(max_daily_doses: 3, min_hours_between_doses: 12, dose_cycle: 'weekly')

      expect(phrase.to_s).to eq('Up to 3 times per week, with at least 12 hours between doses')
    end

    it 'uses singular wording for one dose per cycle' do
      phrase = described_class.new(max_daily_doses: 1, min_hours_between_doses: nil, dose_cycle: 'daily')

      expect(phrase.to_s).to eq('Once per day')
    end

    it 'describes minimum spacing without a dose limit' do
      phrase = described_class.new(max_daily_doses: nil, min_hours_between_doses: 6, dose_cycle: 'daily')

      expect(phrase.to_s).to eq('At least 6 hours between doses')
    end

    it 'describes monthly limits and whole-minute spacing from string inputs' do
      phrase = described_class.new(max_daily_doses: '2', min_hours_between_doses: '0.5', dose_cycle: 'monthly')

      expect(phrase.to_s).to eq('Up to 2 times per month, with at least 30 minutes between doses')
    end

    it 'omits blank and invalid values' do
      phrase = described_class.new(max_daily_doses: '', min_hours_between_doses: 'invalid', dose_cycle: nil)

      expect(phrase.to_s).to eq('')
    end
  end
end
