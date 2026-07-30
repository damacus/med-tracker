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
      'event.dataset' => 'medtracker.opentelemetry'
    )
    expect(record.to_json).not_to include('private failure text')
  end
end
