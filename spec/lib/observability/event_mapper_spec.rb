# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::EventMapper do
  let(:registry) do
    YAML.safe_load_file(Rails.root.join('config/observability/signal_registry.yml'))
  end
  let(:mapping_contracts) do
    {
      'take_attempted.med_tracker' => mapping_contract('schedule', 42, 'medication_take.attempted', 'requested'),
      'take_recorded.med_tracker' => mapping_contract('schedule', 43, 'medication_take.persisted', 'provisional'),
      'take_blocked_by_rules.med_tracker' => blocked_contract,
      'take_errors.med_tracker' => failure_contract,
      'dose_taken.med_tracker' => dose_taken_contract,
      'low_stock_threshold_reached.med_tracker' => low_stock_contract,
      'audit_delivery_backlog.med_tracker' => backlog_contract,
      'rack_attack.throttled' => throttle_contract
    }
  end

  it 'has an explicit mapping for every frozen custom event' do
    expect(event_mapper_class.event_names).to match_array(registry.fetch('custom_events').pluck('name'))
  end

  it 'maps raw medication payloads to bounded safe fields' do
    mapping = event_mapper_class.map('dose_taken.med_tracker', raw_medication_payload)

    expect(mapping).to include(
      name: :medication_take_committed,
      outcome: :success,
      severity: :info,
      reason: :committed,
      attributes: { source_category: :schedule }
    )
    expect(mapping.to_json).not_to match(/41|42|43|tablet|08:04/)
  end

  it 'reduces raw rate-limit network data to a stable throttle category' do
    mapping = event_mapper_class.map(
      'rack_attack.throttled',
      { throttle: 'medication_lookup/ip', ip: '203.0.113.42' }
    )

    expect(mapping.fetch(:attributes)).to eq(throttle_category: :medication_lookup)
    expect(mapping.to_json).not_to include('203.0.113.42')
  end

  it 'builds a privacy-safe canonical event for every frozen custom event' do
    mapping_contracts.each { |domain_name, contract| expect_safe_mapping(domain_name, contract) }
  end

  def event_mapper_class
    Observability::EventMapper
  end

  def raw_medication_payload
    {
      take_id: 41, person_id: 42, medication_id: 43, source_type: 'schedule',
      dose_amount: 2, dose_unit: 'tablet', taken_at: Time.iso8601('2026-07-30T08:04:00+01:00')
    }
  end

  def mapping_contract(source_type, identifier, event_name, reason)
    { payload: { source_type:, person_id: identifier }, event_name:, reason: }
  end

  def blocked_contract
    mapping_contract('schedule', 44, 'medication_take.blocked', 'out_of_stock').tap do |contract|
      contract[:payload].merge!(error: 'out_of_stock', schedule_id: 44)
    end
  end

  def failure_contract
    {
      payload: { source_type: 'schedule', error: 'create_failed', take_id: 45 },
      event_name: 'medication_take.failed',
      reason: 'persistence_failed'
    }
  end

  def dose_taken_contract
    {
      payload: { source_type: 'person_medication', dose_amount: 2, dose_unit: 'tablet' },
      event_name: 'medication_take.committed',
      reason: 'committed'
    }
  end

  def low_stock_contract
    {
      payload: { source_type: 'schedule', medication_id: 46, current_supply: 2 },
      event_name: 'low_stock.threshold_reached',
      reason: 'threshold_reached'
    }
  end

  def backlog_contract
    {
      payload: { severity: 'critical', pending_count: 12, oldest_age_seconds: 90, household_id: 47 },
      event_name: 'audit_delivery.backlog_evaluated',
      reason: 'backlog_critical'
    }
  end

  def throttle_contract
    {
      payload: { throttle: 'medication_lookup/ip', ip: '203.0.113.42' },
      event_name: 'request.throttled',
      reason: 'throttled'
    }
  end

  def expect_safe_mapping(domain_name, contract)
    event = Observability::OperationalEvent.build(
      **event_mapper_class.map(domain_name, contract.fetch(:payload))
    )
    expect_mapped_contract(event, contract)
    expect_private_payload_removed(event)
  end

  def expect_mapped_contract(event, contract)
    expect(event.to_h).to include(
      'event.name' => contract.fetch(:event_name),
      'medtracker.reason' => contract.fetch(:reason)
    )
  end

  def expect_private_payload_removed(event)
    expect(event.to_h.keys).not_to include(*unsafe_keys)
    expect(event.to_json).not_to include('tablet', '203.0.113.42')
  end

  def unsafe_keys
    %w[
      person_id medication_id schedule_id take_id household_id dose_amount dose_unit current_supply ip
    ]
  end
end
