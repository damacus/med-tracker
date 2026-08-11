# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::ProcessLogFormatter do
  it 'gives routine process output a distinct dataset and severity' do
    record = JSON.parse(
      described_class.call('server started on a sensitive endpoint', dataset: 'medtracker.puma')
    )

    expect(record).to include(
      'log.level' => 'info',
      'event.dataset' => 'medtracker.puma',
      'service.name' => 'medtracker'
    )
    expect(record.to_json).not_to include('sensitive endpoint')
  end

  it 'preserves process error severity without unsafe text' do
    record = JSON.parse(
      described_class.call('ERROR: private failure text', dataset: 'medtracker.opentelemetry')
    )

    expect(record).to include(
      'log.level' => 'error',
      'event.dataset' => 'medtracker.opentelemetry',
      'event.reason' => 'export_failed'
    )
    expect(record.to_json).not_to include('private failure text')
    expect(record).not_to have_key('error.type')
  end

  it 'does not derive diagnostic fields from an unknown OpenTelemetry error object' do
    error = RuntimeError.new('private exporter response https://example.test/secret')
    record = JSON.parse(described_class.call(error, dataset: 'medtracker.opentelemetry', severity: 'ERROR'))

    expect(record).to include('event.reason' => 'export_failed')
    expect(record).not_to have_key('error.type')
    expect(record.to_json).not_to include('example.test', 'secret')
  end

  it 'falls back to the minimal process envelope when formatting fails' do
    record = JSON.parse(
      described_class.call('ERROR: private failure text', dataset: 'medtracker.opentelemetry', time: nil)
    )

    expect(record).to include(
      'event.name' => 'process.message',
      'event.dataset' => 'medtracker.opentelemetry',
      'event.reason' => 'export_failed'
    )
    expect(record.to_json).not_to include('private failure text')
  end
end
