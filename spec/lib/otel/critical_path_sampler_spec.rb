# frozen_string_literal: true

require 'rails_helper'
require 'otel/critical_path_sampler'

RSpec.describe Otel::CriticalPathSampler do
  subject(:sampler) { described_class.new(delegate:, critical_path_matchers:) }

  let(:delegate) { OpenTelemetry::SDK::Trace::Samplers::ALWAYS_OFF }
  let(:critical_path_matchers) { ['/medication_takes', 'medication_take.'] }
  let(:trace_id) { '0af7651916cd43dd8448eb211c80319c' }
  let(:base_arguments) do
    {
      trace_id: trace_id,
      parent_context: OpenTelemetry::Context.empty,
      links: [],
      name: 'GET /households/:household_slug/dashboard',
      kind: :server,
      attributes: {}
    }
  end

  it 'samples spans whose HTTP route matches a critical path' do
    result = sampler.should_sample?(
      **base_arguments,
      attributes: { 'http.route' => '/households/:household_slug/schedules/:schedule_id/medication_takes' }
    )

    expect(result).to be_sampled
  end

  it 'samples spans whose name matches a critical model operation' do
    result = sampler.should_sample?(**base_arguments, name: 'medication_take.create')

    expect(result).to be_sampled
  end

  it 'delegates non-critical spans to the configured sampler' do
    result = sampler.should_sample?(**base_arguments)

    expect(result).not_to be_sampled
  end
end
