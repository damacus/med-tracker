# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::DeployedCanary do
  let(:catalogue) do
    YAML.safe_load_file(Rails.root.join('config/observability/production_acceptance_queries.yml'))
  end

  it 'provides bounded Loki checks for every production log acceptance condition' do
    queries = catalogue.dig('loki', 'queries')

    expect(queries.keys).to match_array(expected_loki_queries)
    queries.each_value { |query| expect_bounded_loki_query(query) }
  end

  it 'requires exact trace lookup and bounded canary trace search' do
    expect(catalogue.dig('tempo', 'required')).to be(true)
    expect(catalogue.dig('tempo', 'queries')).to eq(
      'by_trace_id' => '{ trace:id = "$TRACE_ID" }',
      'canary_trace' => '{ resource.service.name = "medtracker" && name = "observability.canary" }'
    )
  end

  it 'defines the fixed canary metric and excludes health-data search keys' do
    expect(catalogue.dig('prometheus', 'canary_metric')).to include(
      'medtracker_observability_canary_total'
    )
    expect(catalogue.to_json).not_to match(
      /person_id|medication_id|schedule_id|household_id|medication_take_id/
    )
  end

  def expected_loki_queries
    %w[
      by_event_id by_workflow_id by_request_id by_job_id by_trace_id deployment_identity
      ingestion_duplicates outer_parser_failures canonical_parser_failures missing_severity
      producer_scoped_counts
    ]
  end

  def expect_bounded_loki_query(query)
    expect(query.fetch('logql')).to include('{namespace="home"')
    expect(query.fetch('window')).to be_in(%w[15m 24h])
  end
end
