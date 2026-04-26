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
    it { expect(described_class.new('weekly').period).to eq(1.week) }
    it { expect(described_class.new('monthly').period).to eq(1.month) }
  end

  describe '#to_s' do
    it { expect(described_class.new('weekly').to_s).to eq('weekly') }
    it { expect(described_class.new('bogus').to_s).to eq('daily') }
  end

  describe 'boundary date edge cases' do
    describe 'exactly at the start of a new cycle (midnight)' do
      let(:midnight) { Time.current.beginning_of_day }

      it 'range_for covers the full new day' do
        expect(described_class.new('daily').range_for(midnight)).to eq(midnight.all_day)
      end

      it 'next_reset_time points to end of that same day + 1 second' do
        expect(described_class.new('daily').next_reset_time(midnight))
          .to be_within(1.second).of(midnight.end_of_day + 1.second)
      end
    end

    describe 'exactly at the end of a cycle (1 second before rollover)' do
      let(:end_of_day) { Time.current.end_of_day }

      it 'range_for still covers the current day' do
        expect(described_class.new('daily').range_for(end_of_day)).to eq(end_of_day.all_day)
      end

      it 'next_reset_time resolves to midnight of the following day' do
        expect(described_class.new('daily').next_reset_time(end_of_day))
          .to be_within(1.second).of(Time.current.beginning_of_day + 1.day)
      end
    end

    describe 'monthly cycle on the 31st in a month with fewer than 31 days' do
      it 'range_for on Feb 28 covers all of February' do
        travel_to Time.zone.local(2025, 2, 28, 12, 0, 0) do
          range = described_class.new('monthly').range_for(Time.current)
          expect(range.first).to eq(Time.zone.local(2025, 2, 28).beginning_of_month)
          expect(range.last).to eq(Time.zone.local(2025, 2, 28).end_of_month)
        end
      end

      it 'next_reset_time on Feb 28 returns Mar 1' do
        travel_to Time.zone.local(2025, 2, 28, 12, 0, 0) do
          reset = described_class.new('monthly').next_reset_time(Time.current)
          expect(reset).to be_within(1.second).of(Time.zone.local(2025, 3, 1).beginning_of_day)
        end
      end
    end

    describe 'cycles crossing a DST spring-forward boundary' do
      it 'daily range_for on the spring-forward day covers the full calendar day' do
        Time.use_zone('Eastern Time (US & Canada)') do
          # 2026-03-08: US clocks spring forward at 02:00 -> 03:00
          travel_to Time.zone.local(2026, 3, 8, 10, 0, 0) do
            range = described_class.new('daily').range_for(Time.current)
            expect(range.first.to_date).to eq(Date.new(2026, 3, 8))
            expect(range.last.to_date).to eq(Date.new(2026, 3, 8))
          end
        end
      end

      it 'daily next_reset_time on the spring-forward day lands on the following day' do
        Time.use_zone('Eastern Time (US & Canada)') do
          travel_to Time.zone.local(2026, 3, 8, 10, 0, 0) do
            reset = described_class.new('daily').next_reset_time(Time.current)
            expect(reset.to_date).to eq(Date.new(2026, 3, 9))
          end
        end
      end
    end
  end
end
