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

    it 'starts a new daily range exactly at midnight' do
      Time.use_zone('Europe/London') do
        time = Time.zone.local(2026, 4, 26, 0, 0, 0)

        expect(described_class.new('daily').range_for(time).begin).to eq(time)
      end
    end

    it 'keeps the last second before midnight in the current daily range' do
      Time.use_zone('Europe/London') do
        time = Time.zone.local(2026, 4, 25, 23, 59, 59)

        expect(described_class.new('daily').range_for(time)).to cover(time)
      end
    end

    it 'covers the final day of shorter months for monthly cycles' do
      Time.use_zone('Europe/London') do
        time = Time.zone.local(2026, 4, 30, 12, 0, 0)
        range = described_class.new('monthly').range_for(time)

        expect(range.begin).to eq(Time.zone.local(2026, 4, 1, 0, 0, 0))
        expect(range.end).to eq(Time.zone.local(2026, 4, 30, 12, 0, 0).end_of_day)
      end
    end

    it 'uses local day boundaries when clocks go forward' do
      Time.use_zone('Europe/London') do
        time = Time.zone.local(2026, 3, 29, 12, 0, 0)
        range = described_class.new('daily').range_for(time)

        expect(range.begin).to eq(Time.zone.local(2026, 3, 29, 0, 0, 0))
        expect(range.end).to eq(Time.zone.local(2026, 3, 29, 12, 0, 0).end_of_day)
      end
    end

    it 'uses local day boundaries when clocks go back' do
      Time.use_zone('Europe/London') do
        time = Time.zone.local(2026, 10, 25, 12, 0, 0)
        range = described_class.new('daily').range_for(time)

        expect(range.begin).to eq(Time.zone.local(2026, 10, 25, 0, 0, 0))
        expect(range.end).to eq(Time.zone.local(2026, 10, 25, 12, 0, 0).end_of_day)
      end
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

    it 'returns midnight for the next day at the end of a daily cycle' do
      Time.use_zone('Europe/London') do
        time = Time.zone.local(2026, 4, 25, 23, 59, 59)

        expect(described_class.new('daily').next_reset_time(time))
          .to be_within(1.second).of(Time.zone.local(2026, 4, 26, 0, 0, 0))
      end
    end

    it 'returns the first day of the next month after a shorter month' do
      Time.use_zone('Europe/London') do
        time = Time.zone.local(2026, 4, 30, 12, 0, 0)

        expect(described_class.new('monthly').next_reset_time(time))
          .to be_within(1.second).of(Time.zone.local(2026, 5, 1, 0, 0, 0))
      end
    end
  end

  describe '#period' do
    it { expect(described_class.new('daily').period).to eq(1.day) }
    it { expect(described_class.new('weekly').period).to eq(1.week) }
    it { expect(described_class.new('monthly').period).to eq(1.month) }
  end

  describe '#to_s' do
    it { expect(described_class.new('weekly').to_s).to eq('weekly') }
    it { expect(described_class.new('bogus').to_s).to eq('daily') }
  end
end
