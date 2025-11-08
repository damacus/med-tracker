# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationTimingHelper do
  describe '#time_until_available' do
    context 'when next_time is nil' do
      it 'returns nil' do
        expect(helper.time_until_available(nil)).to be_nil
      end
    end

    context 'when next_time is in the past' do
      it 'returns "Available now"' do
        next_time = 5.minutes.ago
        expect(helper.time_until_available(next_time)).to eq('Available now')
      end
    end

    context 'when next_time is now' do
      it 'returns "Available now"' do
        next_time = Time.current
        expect(helper.time_until_available(next_time)).to eq('Available now')
      end
    end

    context 'when next_time is less than an hour away' do
      it 'returns minutes only' do
        next_time = 30.minutes.from_now
        expect(helper.time_until_available(next_time)).to eq('Available in 30m')
      end

      it 'rounds down partial minutes' do
        next_time = 30.minutes.from_now + 30.seconds
        expect(helper.time_until_available(next_time)).to eq('Available in 30m')
      end
    end

    context 'when next_time is hours away with no remaining minutes' do
      it 'returns hours only' do
        next_time = 2.hours.from_now
        expect(helper.time_until_available(next_time)).to eq('Available in 2h')
      end
    end

    context 'when next_time is hours and minutes away' do
      it 'returns both hours and minutes' do
        next_time = 2.hours.from_now + 30.minutes
        expect(helper.time_until_available(next_time)).to eq('Available in 2h 30m')
      end

      it 'handles single hour and minutes' do
        next_time = 1.hour.from_now + 15.minutes
        expect(helper.time_until_available(next_time)).to eq('Available in 1h 15m')
      end

      it 'rounds down partial minutes' do
        next_time = 3.hours.from_now + 45.minutes + 45.seconds
        expect(helper.time_until_available(next_time)).to eq('Available in 3h 45m')
      end
    end

    context 'when next_time is many hours away' do
      it 'formats correctly for long durations' do
        next_time = 23.hours.from_now + 59.minutes
        expect(helper.time_until_available(next_time)).to eq('Available in 23h 59m')
      end
    end
  end
end
