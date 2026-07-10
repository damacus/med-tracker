# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Audit::EntryFilter do
  it 'filters by household and inclusive ISO 8601 time bounds' do
    relation = instance_double(ActiveRecord::Relation)
    household_relation = instance_double(ActiveRecord::Relation)
    from_relation = instance_double(ActiveRecord::Relation)
    from = Time.iso8601('2026-07-01T00:00:00Z')
    to = Time.iso8601('2026-07-02T00:00:00Z')
    allow(relation).to receive(:where).with(household_id: 42).and_return(household_relation)
    allow(household_relation).to receive(:where).with(occurred_at: from..).and_return(from_relation)
    allow(from_relation).to receive(:where).with(occurred_at: ..to).and_return(:filtered)

    result = described_class.new(
      { 'HOUSEHOLD_ID' => '42', 'FROM' => from.iso8601, 'TO' => to.iso8601 }, relation:
    ).call

    expect(result).to eq(:filtered)
  end

  it 'rejects invalid household and time filters' do
    expect do
      described_class.new({ 'HOUSEHOLD_ID' => 'not-an-id' }).call
    end.to raise_error(described_class::Invalid, 'invalid HOUSEHOLD_ID')

    expect do
      described_class.new({ 'FROM' => 'yesterday' }).call
    end.to raise_error(described_class::Invalid, 'invalid FROM')
  end
end
