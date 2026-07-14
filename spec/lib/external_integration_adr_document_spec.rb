# frozen_string_literal: true

require 'rails_helper'

class ExternalIntegrationAdrDocument
  def previous = Rails.root.join('docs/adrs/0007-external-app-integration-contract.md').read

  def current = Rails.root.join('docs/adrs/0010-external-integration-architecture.md').read

  def index = Rails.root.join('docs/index.md').read
end

RSpec.describe ExternalIntegrationAdrDocument do
  subject(:document) { described_class.new }

  it 'supersedes the contradictory external integration decision' do
    expect(document.previous).to include(
      '- Status: Superseded by [ADR 0010](0010-external-integration-architecture.md)'
    )
  end

  it 'records the shipped integration and SMART authorization architecture' do
    expect(document.current.squish).to include(
      '- Status: Accepted',
      '/api/v1',
      '/mcp',
      '/api/fhir/R4',
      'standalone launch',
      'PKCE with `S256`',
      '15 minutes',
      '30 days',
      'SMART resource scope',
      'MedTracker policy scope',
      'Dynamic client registration is not supported'
    )
  end

  it 'makes the current decision discoverable' do
    expect(document.index).to include(
      '[External Integration Architecture](adrs/0010-external-integration-architecture.md)'
    )
  end
end
