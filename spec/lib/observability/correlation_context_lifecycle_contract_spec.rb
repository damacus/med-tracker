# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::CorrelationContext do
  let(:now) { Time.iso8601('2026-07-30T09:30:00Z') }
  let(:lifetime) { 24.hours }

  it 'starts an opaque bounded-lifetime workflow without root causation' do
    context = correlation_context_class.start(now:, lifetime:)

    expect(context.workflow_id).to match(uuid_pattern)
    expect(context.causation_id).to be_nil
    expect(context.attempt_id).to be_nil
    expect(context.expires_at).to eq(now + lifetime)
  end

  it 'propagates a workflow across a job boundary without domain identifiers' do
    context = correlation_context_class.start(now:, lifetime:)
    payload = context.to_propagation
    restored = correlation_context_class.from_propagation(payload, now: now + 1.hour)

    expect(restored.workflow_id).to eq(context.workflow_id)
    expect(payload.keys).to contain_exactly('workflow.id', 'causation.id', 'attempt.id', 'expires_at')
    expect(payload.to_json).not_to match(/person|medication|schedule|household|notification/)
  end

  it 'links a non-root event only to its immediate causal event' do
    context = correlation_context_class.start(now:, lifetime:)
    caused = context.caused_by('43d1e299-df0d-4c7f-89d0-61706b895442')

    expect(caused.workflow_id).to eq(context.workflow_id)
    expect(caused.causation_id).to eq('43d1e299-df0d-4c7f-89d0-61706b895442')
    expect(caused.attempt_id).to be_nil
  end

  it 'rotates attempt identifiers for retries while retaining workflow and causation' do
    root = correlation_context_class.start(now:, lifetime:)
                                    .caused_by('43d1e299-df0d-4c7f-89d0-61706b895442')
    first_attempt = root.next_attempt
    retry_attempt = root.next_attempt

    aggregate_failures do
      expect(first_attempt.workflow_id).to eq(root.workflow_id)
      expect(retry_attempt.workflow_id).to eq(root.workflow_id)
      expect(first_attempt.causation_id).to eq(root.causation_id)
      expect(retry_attempt.causation_id).to eq(root.causation_id)
      expect(first_attempt.attempt_id).to match(uuid_pattern)
      expect(retry_attempt.attempt_id).not_to eq(first_attempt.attempt_id)
    end
  end

  it 'rejects expired propagation and starts a fresh uncaused workflow' do
    original = correlation_context_class.start(now:, lifetime:)
                                        .caused_by('43d1e299-df0d-4c7f-89d0-61706b895442')
                                        .next_attempt
    restored = correlation_context_class.from_propagation(
      original.to_propagation,
      now: now + lifetime + 1.second,
      rotate_expired: true
    )

    expect(restored.workflow_id).not_to eq(original.workflow_id)
    expect(restored.causation_id).to be_nil
    expect(restored.attempt_id).to be_nil
  end

  it 'omits invalid propagated identifiers instead of inventing placeholders' do
    restored = correlation_context_class.from_propagation(
      {
        'workflow.id' => 'household-42',
        'causation.id' => 'unknown',
        'attempt.id' => '',
        'expires_at' => (now + lifetime).iso8601
      },
      now:
    )

    expect(restored.workflow_id).to match(uuid_pattern)
    expect(restored.causation_id).to be_nil
    expect(restored.attempt_id).to be_nil
  end

  def correlation_context_class
    Observability::CorrelationContext
  end

  def uuid_pattern
    /\A[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}\z/
  end
end
