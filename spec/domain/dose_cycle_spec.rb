# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DoseCycle do
  let(:now) { Time.current }

  describe '#range_for' do
    it 'returns all_day for daily' do
      expect(described_class.new('daily').range_for(now)).to eq(now.all_day)
    end

    it 'returns all_week for weekly' do
      expect(described_class.new('weekly').range_for(now)).to eq(now.all_week)
    end

    it 'returns all_month for monthly' do
      expect(described_class.new('monthly').range_for(now)).to eq(now.all_month)
    end

    it 'defaults to all_day for unknown value' do
      expect(described_class.new('bogus').range_for(now)).to eq(now.all_day)
    end
  end

  describe '#next_reset_time' do
    it 'returns end_of_day + 1s for daily' do
      expect(described_class.new('daily').next_reset_time(now)).to be_within(1.second).of(now.end_of_day + 1.second)
    end

    it 'returns end_of_week + 1s for weekly' do
      expect(described_class.new('weekly').next_reset_time(now)).to be_within(1.second).of(now.end_of_week + 1.second)
    end

    it 'returns end_of_month + 1s for monthly' do
      expect(described_class.new('monthly').next_reset_time(now)).to be_within(1.second).of(now.end_of_month + 1.second)
    end
  end

  describe '#period' do
    it { expect(described_class.new('daily').period).to eq(1.day) }
    it { expect(described_class.new('weekly').period).to eq(7.days) }
    it { expect(described_class.new('monthly').period).to eq(30.days) }
  end

  describe '#to_s' do
    it { expect(described_class.new('weekly').to_s).to eq('weekly') }
    it { expect(described_class.new('bogus').to_s).to eq('daily') }
  end
end
