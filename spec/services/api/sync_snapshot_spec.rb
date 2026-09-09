# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::SyncSnapshot do
  it 'locks the household and captures the cursor before exporting records' do
    household = instance_double(Household)
    exporter = instance_double(PortableData::Exporter)
    cursor = Time.zone.parse('2026-07-13 12:00:00')

    allow(household).to receive(:lock!)
    allow(Time).to receive(:current).and_return(cursor)
    allow(exporter).to receive(:mobile_v2_payload).and_return(records: { medication_pause_periods: [{ id: 'pause-1' }] })

    payload = described_class.new(household: household, exporter: exporter).payload

    expect(household).to have_received(:lock!).ordered
    expect(Time).to have_received(:current).ordered
    expect(exporter).to have_received(:mobile_v2_payload).ordered
    expect(payload).to include(
      format: 'medtracker.portable.v2', cursor: cursor.iso8601,
      records: { medication_pause_periods: [{ id: 'pause-1' }] }
    )
  end
end
