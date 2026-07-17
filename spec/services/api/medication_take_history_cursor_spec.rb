# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::MedicationTakeHistoryCursor do
  let(:household) { create(:household) }
  let(:taken_at) { Time.iso8601('2026-07-17T12:00:00Z') }
  let(:take) { instance_double(MedicationTake, id: 42, taken_at: taken_at) }
  let(:filter_digest) { Digest::SHA256.hexdigest('filters') }

  it 'round trips an encrypted position only for the original household and filters' do
    token = described_class.new(household: household).encode(take, filter_digest: filter_digest)

    expect(token).not_to include(taken_at.iso8601)
    expect(
      described_class.new(household: household).decode(token, filter_digest: filter_digest)
    ).to eq([taken_at, 42])
    expect do
      described_class.new(household: create(:household)).decode(token, filter_digest: filter_digest)
    end.to raise_error(described_class::Invalid)
  end

  it 'expires cursor positions after the bounded lifetime' do
    reference_time = Time.iso8601('2026-07-17T12:00:00Z')
    token = nil
    travel_to(reference_time) do
      token = described_class.new(household: household).encode(take, filter_digest: filter_digest)
    end

    travel_to(reference_time + 2.hours) do
      expect do
        described_class.new(household: household).decode(token, filter_digest: filter_digest)
      end.to raise_error(described_class::Invalid)
    end
  end
end
