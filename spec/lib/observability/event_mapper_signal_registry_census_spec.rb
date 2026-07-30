# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::EventMapper do
  let(:registry) do
    YAML.safe_load_file(Rails.root.join('config/observability/signal_registry.yml'))
  end
  let(:producer_fields) do
    %w[source formatter transport sink privacy_classification owner verification]
  end
  let(:event_fields) do
    %w[
      publisher payload_contract production_subscribers operational_sink disposition
      correlation_context transaction_timing failure_policy privacy_classification owner verification
    ]
  end
  let(:logger_policy_fields) do
    %w[
      disposition message_contract correlation_context transaction_timing failure_policy
      privacy_classification owner verification
    ]
  end
  let(:workflow_fields) do
    %w[
      entrypoints decisions side_effects retries existing_signals correlation_context
      privacy_classification missing_outcomes owner verification
    ]
  end

  it 'pins the implementation baseline to the reviewed source revision' do
    expect(registry.fetch('source_revision')).to eq('82970afde0f90c40058f1b8fd01b7e3276f637c5')
  end

  it 'enumerates every production output producer' do
    expect(registry.fetch('output_producers').pluck('id')).to contain_exactly(
      'thruster',
      'rails_ecs_lograge',
      'puma',
      'solid_queue',
      'opentelemetry_sdk'
    )
  end

  it 'enumerates every custom event type in the frozen baseline' do
    expect(registry.fetch('custom_events').pluck('name')).to match_array(discovered_custom_events)
    expect(discovered_custom_events.size).to eq(8)
  end

  it 'preserves every frozen direct logger site and leaves no unsafe direct logger calls' do
    registered_sites = registry.fetch('direct_logger_sites').map do |site|
      "#{site.fetch('path')}:#{site.fetch('line')}:#{site.fetch('level')}"
    end

    expect(registered_sites.size).to eq(50)
    expect(registered_sites).to all(match(%r{\A(?:app|config|lib)/.+:\d+:(?:debug|info|warn|error|fatal)\z}))
    expect(discovered_direct_logger_sites).to be_empty
  end

  it 'enumerates the bounded workflow families' do
    expect(registry.fetch('workflows').pluck('id')).to contain_exactly(
      'http_request_completion',
      'medication_administration',
      'scheduled_reminders',
      'low_stock_notifications',
      'push_delivery',
      'rate_limiting',
      'audit_backlog_monitoring'
    )
  end

  it 'records the production transport and ownership contract for every producer' do
    expect(registry.fetch('transports')).not_to be_empty

    registry.fetch('output_producers').each do |producer|
      expect(producer).to include(*producer_fields), producer.fetch('id')
      expect(registry.fetch('transports')).to have_key(producer.fetch('transport'))
    end
  end

  it 'records a complete operational mapping for every custom event' do
    expect(registry.fetch('custom_events')).to all(include(*event_fields))
  end

  it 'assigns every direct logger site a complete reviewed disposition policy' do
    policies = registry.fetch('direct_logger_policies')

    registry.fetch('direct_logger_sites').each do |site|
      policy_id = site.fetch('policy')
      expect(policies).to have_key(policy_id), "#{site.fetch('path')}:#{site.fetch('line')}"
      expect(policies.fetch(policy_id)).to include(*logger_policy_fields), policy_id
    end
  end

  it 'records the complete bounded workflow matrix' do
    expect(registry.fetch('workflows')).to all(include(*workflow_fields))
  end

  it 'names out-of-baseline discoveries as owned follow-up changes' do
    registry.fetch('follow_up_changes').each do |follow_up|
      expect(follow_up).to include('id', 'discovery', 'owner', 'scope_boundary')
      expect(registry.fetch('workflows').pluck('id')).not_to include(follow_up.fetch('id'))
    end
  end

  def discovered_custom_events
    production_ruby_sources
      .flat_map { |source| source.scan(/['"]([a-z_]+\.med_tracker|rack_attack\.throttled)['"]/) }
      .flatten
      .uniq
  end

  def discovered_direct_logger_sites
    production_ruby_files.flat_map do |path|
      relative_path = path.relative_path_from(Rails.root)

      path.each_line.with_index(1).filter_map do |line, line_number|
        match = line.match(/Rails\.logger\.(debug|info|warn|error|fatal)\b/)
        "#{relative_path}:#{line_number}:#{match[1]}" if match
      end
    end
  end

  def production_ruby_sources
    production_ruby_files.map(&:read)
  end

  def production_ruby_files
    @production_ruby_files ||= %w[app config lib].flat_map do |directory|
      Rails.root.glob("#{directory}/**/*.rb")
    end
  end
end
