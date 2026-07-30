# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::NotificationStage do
  before { allow(Observability::Publisher).to receive(:emit) }

  it 'emits a bounded reminder outcome without domain identifiers or message content' do
    emit_missed_dose_notification

    expect(Observability::Publisher).to have_received(:emit).with(
      name: :notification_stage,
      outcome: :success,
      severity: :info,
      reason: :eligible,
      attributes: {
        notification_kind: :missed_dose,
        workflow_stage: :missed_dose_evaluation,
        recipient_count: 2
      }
    )
  end

  it 'uses unknown rather than success when provider delivery is not confirmed' do
    described_class.emit(
      kind: :dose_due,
      stage: :provider_outcome,
      reason: :provider_accepted,
      channel: :web_push,
      provider: :web_push
    )

    expect(Observability::Publisher).to have_received(:emit).with(
      hash_including(outcome: :unknown, reason: :provider_accepted)
    )
  end

  def emit_missed_dose_notification
    described_class.emit(
      kind: :missed_dose,
      stage: :missed_dose_evaluation,
      reason: :eligible,
      recipient_count: 2,
      person_id: 42,
      body: 'Daniel may have missed a dose'
    )
  end
end
