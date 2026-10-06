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

  it 'emits a fixed error type for an allowlisted exporter transport error' do
    error = OpenSSL::SSL::SSLError.new('private exporter response https://example.test/secret')
    record = JSON.parse(described_class.call(error, dataset: 'medtracker.opentelemetry', severity: 'ERROR'))

    expect(record).to include(
      'event.reason' => 'export_failed',
      'error.type' => 'OpenSSL::SSL::SSLError'
    )
    expect(record.to_json).not_to include('example.test', 'secret')
  end

  it 'limits allowlisted error types to OpenTelemetry error records' do
    error = OpenSSL::SSL::SSLError.new('private endpoint')
    puma_record = JSON.parse(described_class.call(error, dataset: 'medtracker.puma', severity: 'ERROR'))
    info_record = JSON.parse(described_class.call(error, dataset: 'medtracker.opentelemetry', severity: 'INFO'))

    expect(puma_record).not_to have_key('error.type')
    expect(info_record).not_to have_key('error.type')
    expect([puma_record, info_record].to_json).not_to include('private endpoint')
  end

  it 'does not expose an unknown exception class as an error type' do
    unknown_error_class = stub_const('UnknownExporterError', Class.new(StandardError))
    record = JSON.parse(
      described_class.call(unknown_error_class.new('private endpoint'),
                           dataset: 'medtracker.opentelemetry', severity: 'ERROR')
    )

    expect(record).to include('event.reason' => 'export_failed')
    expect(record).not_to have_key('error.type')
    expect(record.to_json).not_to include('private endpoint', unknown_error_class.name.to_s)
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
    expect(record).not_to have_key('error.type')
    expect(record.to_json).not_to include('private failure text')
  end
end
