# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::ConsistentSyncRead do
  it 'locks the household and captures the cursor before reading sync data' do
    household = instance_double(Household)
    cursor = Time.zone.parse('2026-07-13 12:00:00')
    reader = proc { { records: [] } }

    allow(household).to receive(:lock!)
    allow(Time).to receive(:current).and_return(cursor)
    allow(reader).to receive(:call).and_call_original

    payload = described_class.new(household: household).call do |captured_cursor|
      reader.call.merge(cursor: captured_cursor)
    end

    expect(household).to have_received(:lock!).ordered
    expect(Time).to have_received(:current).ordered
    expect(reader).to have_received(:call).ordered
    expect(payload).to eq(records: [], cursor: cursor.iso8601)
  end
end
