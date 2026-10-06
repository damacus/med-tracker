# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::OperationalEvent do
  let(:runbook) { Rails.root.join('docs/operations/application-observability.md').read }
  let(:adr) { Rails.root.join('docs/adrs/0004-domain-events-with-active-support-notifications.md').read }

  it 'documents the complete operational contract' do
    expect(runbook).to include(
      'Canonical event schema',
      'Correlation identifiers',
      'Privacy contract',
      'Severity policy',
      'Transaction truth',
      'Failure isolation',
      'Ownership',
      'Rollback and compatibility',
      'Verification gates'
    )
  end

  it 'keeps production acceptance separate from pre-deployment verification' do
    expect(runbook).to include('does not complete the OpenSpec change')
    expect(runbook).to include('exact immutable image digest')
    expect(runbook).to include('twenty-four-hour')
  end

  it 'amends ADR 0004 without replacing Active Support domain events' do
    expect(adr).to include('Follow-up: Operational signals')
    expect(adr).to include('ActiveSupport::Notifications remains the in-process domain-event mechanism')
    expect(adr).to include('does not replace, wrap, or become a subscriber framework')
  end
end
