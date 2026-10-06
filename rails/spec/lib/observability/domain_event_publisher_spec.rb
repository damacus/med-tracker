# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::DomainEventPublisher do
  let(:event_name) { 'take_attempted.med_tracker' }
  let(:payload) { { source_type: 'schedule', person_id: 42 } }

  before do
    allow(Observability::Publisher).to receive(:emit)
  end

  it 'records publication before dispatching to Active Support subscribers' do
    publication_was_recorded = false
    allow(Observability::Publisher).to receive(:emit) do |name:, **|
      publication_was_recorded = true if name == event_name
    end
    subscriber = ->(*) { expect(publication_was_recorded).to be(true) }

    ActiveSupport::Notifications.subscribed(subscriber, event_name) do
      described_class.instrument(event_name, payload)
    end

    expect(Observability::Publisher).to have_received(:emit).with(name: event_name, **payload).once
  end

  it 'does not let an operational logging failure block domain dispatch' do
    allow(Observability::Publisher).to receive(:emit).and_raise(IOError, 'logging unavailable')
    subscriber = instance_spy(Proc)

    ActiveSupport::Notifications.subscribed(subscriber, event_name) do
      expect { described_class.instrument(event_name, payload) }.not_to raise_error
    end

    expect(subscriber).to have_received(:call)
  end

  it 'records subscriber failure and preserves its propagation' do
    subscriber = ->(*) { raise ArgumentError, 'private subscriber failure' }

    expect do
      ActiveSupport::Notifications.subscribed(subscriber, event_name) do
        described_class.instrument(event_name, payload)
      end
    end.to raise_error(ArgumentError, 'private subscriber failure')

    expect(Observability::Publisher).to have_received(:emit).with(
      name: :domain_event_subscriber_failed,
      outcome: :failure,
      severity: :error,
      reason: :subscriber_failed,
      attributes: { workflow_stage: :take_attempted },
      error_type: ArgumentError
    ).once
  end
end
