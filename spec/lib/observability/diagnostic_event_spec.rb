# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::DiagnosticEvent do
  before { allow(Observability::Publisher).to receive(:emit) }

  it 'replaces free text with component, stable reason, severity, and error type' do
    described_class.emit(
      component: :external_lookup,
      reason: :operation_failed,
      severity: :error,
      error: RuntimeError.new('private upstream response')
    )

    expect(Observability::Publisher).to have_received(:emit).with(
      name: :diagnostic,
      outcome: :failure,
      severity: :error,
      reason: :operation_failed,
      attributes: { diagnostic_component: :external_lookup },
      error_type: RuntimeError
    )
  end

  it 'records configuration success without accepting source message text' do
    described_class.emit(
      component: :oidc,
      reason: :configured,
      severity: :info,
      message: 'issuer and client secret'
    )

    expect(Observability::Publisher).to have_received(:emit).with(
      name: :diagnostic,
      outcome: :success,
      severity: :info,
      reason: :configured,
      attributes: { diagnostic_component: :oidc },
      error_type: nil
    )
  end
end
