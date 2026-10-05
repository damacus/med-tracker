# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::RequestEvent do
  let(:successful_event) do
    described_class.from(
      method: 'GET',
      route: '/households/:household_slug/dashboard(.:format)',
      status: 200,
      duration_ms: 12.345,
      request_id: 'request-opaque'
    )
  end

  it 'maps a successful request to a canonical safe envelope' do
    expect(successful_event.to_h).to include(
      'event.name' => 'http.request.completed',
      'event.outcome' => 'success',
      'log.level' => 'info',
      'http.request.method' => 'GET',
      'http.route' => '/households/:household_slug/dashboard(.:format)',
      'http.response.status_code' => 200,
      'event.duration' => 12_345_000,
      'medtracker.request.id' => 'request-opaque'
    )
  end

  it 'maps server failures to error severity and safe exception identity' do
    event = described_class.from(
      method: 'POST',
      route: '/medication_takes',
      status: 500,
      duration_ms: 8,
      request_id: 'request-opaque',
      error_type: ActiveRecord::RecordNotSaved
    )

    expect(event.to_h).to include(
      'event.outcome' => 'failure',
      'log.level' => 'error',
      'error.type' => 'ActiveRecord::RecordNotSaved'
    )
  end
end
