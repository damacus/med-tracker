# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::Sync::MedicationTakeOperation do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules,
           :medication_takes

  subject(:operation) { described_class.new }

  let(:source) { schedules(:jane_ibuprofen) }
  let(:user) { users(:admin) }
  let(:attributes) do
    {
      client_uuid: SecureRandom.uuid,
      source_type: 'schedule',
      source_id: source.portable_id,
      taken_at: 2.days.from_now.iso8601,
      dose_amount: nil,
      taken_from_medication_id: source.medication_id
    }
  end
  let(:dose_service) { instance_double(MedicationAdministration::RecordDose) }

  before do
    allow(MedicationAdministration::RecordDose).to receive(:new).and_return(dose_service)
  end

  it 'delegates new takes to the canonical medication administration service' do
    take = medication_takes(:jane_morning_ibuprofen)
    allow(dose_service).to receive(:call).and_return(
      MedicationAdministration::RecordDose::Result.new(success: true, take: take, error: nil)
    )

    result = operation.call(attributes: attributes, user: user, route: '/sync') { source }

    expect(result).to have_attributes(take: take, replayed: false)
    expect(dose_service).to have_received(:call).with(
      source: source,
      amount_override: nil,
      taken_from_medication_id: source.medication_id,
      user: user,
      taken_at: Time.zone.parse(attributes.fetch(:taken_at)),
      client_uuid: attributes.fetch(:client_uuid),
      route: '/sync'
    )
  end

  it 'returns authorized existing takes as replays without recording another dose' do
    take = medication_takes(:jane_morning_ibuprofen)

    result = operation.call(attributes: attributes, user: user, existing_take: take) { raise 'source resolved' }

    expect(result).to have_attributes(take: take, replayed: true)
    expect(MedicationAdministration::RecordDose).not_to have_received(:new)
  end

  it 'rejects missing UUIDs, invalid timestamps, and incomplete source references' do
    invalid_inputs = {
      attributes.merge(client_uuid: '') => 'client_uuid is required',
      attributes.merge(taken_at: 'private-dose-time') => 'taken_at is invalid',
      attributes.except(:source_type) => 'source_type and source_id are required',
      attributes.except(:source_id) => 'source_type and source_id are required'
    }

    invalid_inputs.each do |input, message|
      expect { operation.call(attributes: input, user: user) { source } }
        .to raise_error(described_class::Error) do |error|
          expect(error).to have_attributes(code: 'medication_take_invalid', status: :unprocessable_content)
          expect(error.message).to eq(message)
          sensitive_values = [input[:client_uuid], input[:source_id]].compact_blank
          expect(error.message).not_to include(*sensitive_values) if sensitive_values.any?
        end
    end
  end

  it 'maps medication safety failures to stable public errors' do
    expect(domain_error_for(:out_of_stock)).to have_attributes(code: 'medication_stock_unavailable')
    expect(domain_error_for(:cooldown)).to have_attributes(code: 'medication_timing_conflict')
    expect(domain_error_for(:paused)).to have_attributes(code: 'medication_timing_conflict')
    expect(domain_error_for(:overlapping_prescription_restriction))
      .to have_attributes(code: 'medication_timing_conflict')
  end

  it 'maps other domain failures to an invalid take error' do
    %i[invalid_amount selection_required invalid_source household_unavailable].each do |domain_error|
      expect(domain_error_for(domain_error)).to have_attributes(code: 'medication_take_invalid')
    end
  end

  it 'signals a whole-batch retry for a serialized persistence race' do
    allow(dose_service).to receive(:call).and_return(
      MedicationAdministration::RecordDose::Result.new(success: false, take: nil, error: :create_failed)
    )

    expect { operation.call(attributes: attributes, user: user) { source } }
      .to raise_error(described_class::RetryBatch) do |error|
        expect(error.reason).to eq(:persistence_failure)
      end
  end

  it 'signals a whole-batch retry only for the client UUID constraint' do
    matching_error = ActiveRecord::RecordNotUnique.new(
      'duplicate key violates unique constraint "index_medication_takes_on_client_uuid"'
    )
    other_error = ActiveRecord::RecordNotUnique.new(
      'duplicate key violates unique constraint "index_medication_takes_on_household_id_and_portable_id"'
    )

    allow(dose_service).to receive(:call).and_raise(matching_error)
    expect { operation.call(attributes: attributes, user: user) { source } }
      .to raise_error(described_class::RetryBatch) do |error|
        expect(error).to be_client_uuid_constraint
      end

    allow(dose_service).to receive(:call).and_raise(other_error)
    expect { operation.call(attributes: attributes, user: user) { source } }
      .to raise_error(other_error)
  end

  def domain_error_for(domain_error)
    allow(dose_service).to receive(:call).and_return(
      MedicationAdministration::RecordDose::Result.new(success: false, take: nil, error: domain_error)
    )

    operation.call(attributes: attributes, user: user) { source }
  rescue described_class::Error => e
    e
  end
end
