# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::RequestCompletionSubscriber do
  let(:request) do
    instance_double(
      ActionDispatch::Request,
      request_method: 'GET',
      route_uri_pattern: '/households/:household_slug/dashboard(.:format)',
      request_id: 'request-opaque'
    )
  end
  let(:notification) do
    instance_double(
      ActiveSupport::Notifications::Event,
      duration: 12.5,
      payload: {
        request:,
        status: 200,
        path: '/households/private-family/dashboard?person=Daniel'
      }
    )
  end

  before { allow(Observability::Publisher).to receive(:publish) }

  it 'publishes exactly one request completion using the route template' do
    described_class.call(notification)

    expect(Observability::Publisher).to have_received(:publish).once do |event|
      expect(event.to_h).to include(
        'event.name' => 'http.request.completed',
        'http.route' => '/households/:household_slug/dashboard(.:format)'
      )
      expect(event.to_json).not_to include('private-family', 'Daniel')
    end
  end

  it 'uses a bounded Rodauth action route when middleware handled the request' do
    allow(request).to receive(:route_uri_pattern).and_return(nil)
    notification.payload[:controller] = 'RodauthController'
    notification.payload[:action] = 'login'

    described_class.call(notification)

    expect(Observability::Publisher).to have_received(:publish).once do |event|
      expect(event.to_h).to include('http.route' => '/login')
    end
  end

  it 'suppresses routine successful health checks' do
    allow(request).to receive(:route_uri_pattern).and_return('/up')

    described_class.call(notification)

    expect(Observability::Publisher).not_to have_received(:publish)
  end

  it 'preserves failed health checks' do
    allow(request).to receive(:route_uri_pattern).and_return('/up')
    notification.payload[:status] = 500

    described_class.call(notification)

    expect(Observability::Publisher).to have_received(:publish).once
  end

  it 'maps escaped exceptions to a server failure when Rails omits the status' do
    notification.payload.delete(:status)
    notification.payload[:exception_object] = RuntimeError.new('private failure text')

    described_class.call(notification)

    expect(Observability::Publisher).to have_received(:publish).once do |event|
      expect(event.to_h).to include(
        'event.outcome' => 'failure',
        'http.response.status_code' => 500,
        'error.type' => 'RuntimeError'
      )
      expect(event.to_json).not_to include('private failure text')
    end
  end
end
