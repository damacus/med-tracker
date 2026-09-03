# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationPausePeriod do
  fixtures :households, :accounts, :people, :locations, :medications, :dosages, :schedules,
           :person_medications

  subject(:pause_period) do
    described_class.new(
      schedule: schedules(:john_paracetamol),
      reason: 'clinician_advice',
      started_at: Time.current,
      recorded_by_membership: membership
    )
  end

  let(:household) { households(:fixture_household) }
  let(:membership) do
    household.household_memberships.find_or_create_by!(account: accounts(:admin)) do |record|
      record.role = :owner
      record.status = :active
    end
  end

  describe 'validations' do
    it 'accepts each public pause reason' do
      described_class::PUBLIC_REASONS.each do |reason|
        expect(pause_period.tap { it.reason = reason }).to be_valid
      end
    end

    it 'requires exactly one source' do
      pause_period.person_medication = person_medications(:john_vitamin_d)

      expect(pause_period).not_to be_valid
      expect(pause_period.errors[:base]).to include('Must have exactly one source')
    end

    it 'rejects an unsupported reason' do
      pause_period.reason = 'forgotten'

      expect(pause_period).not_to be_valid
      expect(pause_period.errors[:reason]).to include('is not included in the list')
    end

    it 'requires current pauses to have a known start and recording actor' do
      pause_period.started_at = nil
      pause_period.recorded_by_membership = nil

      expect(pause_period).not_to be_valid
      expect(pause_period.errors[:started_at]).to include("can't be blank")
      expect(pause_period.errors[:recorded_by_membership]).to include("can't be blank")
    end

    it 'permits unknown legacy context without inventing a start or actor' do
      pause_period.assign_attributes(
        legacy_context: true,
        reason: 'reason_not_recorded',
        started_at: nil,
        recorded_by_membership: nil
      )

      expect(pause_period).to be_valid
    end

    it 'rejects an end before a known start' do
      pause_period.ended_at = pause_period.started_at - 1.second
      pause_period.resumed_by_membership = membership

      expect(pause_period).not_to be_valid
      expect(pause_period.errors[:ended_at]).to include('must be on or after the start time')
    end

    it 'requires an ending actor when the period is closed' do
      pause_period.ended_at = pause_period.started_at + 1.hour

      expect(pause_period).not_to be_valid
      expect(pause_period.errors[:resumed_by_membership]).to include("can't be blank")
    end

    it 'rejects an ending actor while the period remains open' do
      pause_period.resumed_by_membership = membership

      expect(pause_period).not_to be_valid
      expect(pause_period.errors[:resumed_by_membership]).to include('must be blank')
    end

    it 'rejects source and actor records from another household' do
      other_membership = HouseholdMembership.new(household_id: household.id + 1)
      pause_period.recorded_by_membership = other_membership

      expect(pause_period).not_to be_valid
      expect(pause_period.errors[:recorded_by_membership]).to include('must belong to the same household')
    end
  end

  describe 'persistence' do
    it 'assigns the source household and a portable identifier' do
      pause_period.save!

      expect(pause_period).to have_attributes(household: household)
      expect(pause_period.portable_id).to be_present
    end

    it 'keeps portable identifiers unique inside a household' do
      first_period = pause_period.tap(&:save!)
      duplicate = described_class.new(
        person_medication: person_medications(:john_vitamin_d),
        reason: 'side_effects',
        started_at: Time.current,
        recorded_by_membership: membership,
        portable_id: first_period.portable_id
      )

      expect(duplicate).not_to be_valid
      expect(duplicate.errors[:portable_id]).to include('has already been taken')
    end

    it 'allows only one open period for each source type' do
      expect_duplicate_open_period_rejected(schedule: schedules(:john_paracetamol))
      expect_duplicate_open_period_rejected(person_medication: person_medications(:john_vitamin_d))
    end

    it 'records an audit version' do
      expect { pause_period.save! }.to change(PaperTrail::Version, :count).by(1)
    end
  end

  describe 'database constraints' do
    let(:connection) { ActiveRecord::Base.connection }
    let(:constraints) { connection.check_constraints(:medication_pause_periods).index_by(&:name) }

    it 'enforces exact source, interval, legacy, and actor rules' do
      expect(constraints.fetch('chk_medication_pause_periods_exactly_one_source').expression)
        .to include('num_nonnulls')
      expect(constraints.fetch('chk_medication_pause_periods_interval').expression).to include('ended_at')
      expect(constraints.fetch('chk_medication_pause_periods_legacy_context').expression)
        .to include('reason_not_recorded')
      expect(constraints.fetch('chk_medication_pause_periods_resuming_actor').expression)
        .to include('resumed_by_membership_id')
    end

    it 'uses composite household foreign keys for sources and actors' do
      composite_columns = connection.foreign_keys(:medication_pause_periods).filter_map do |foreign_key|
        foreign_key.options[:column] if Array(foreign_key.options[:column]).include?('household_id')
      end

      expect(composite_columns).to include(
        %w[schedule_id household_id],
        %w[person_medication_id household_id],
        %w[recorded_by_membership_id household_id],
        %w[resumed_by_membership_id household_id]
      )
    end

    it 'enables and forces the household tenant isolation policy' do
      row_security = connection.select_one(<<~SQL.squish)
        SELECT relrowsecurity, relforcerowsecurity
        FROM pg_class
        WHERE oid = 'medication_pause_periods'::regclass
      SQL
      policies = connection.select_values(<<~SQL.squish)
        SELECT polname
        FROM pg_policy
        WHERE polrelid = 'medication_pause_periods'::regclass
      SQL

      expect(row_security).to include('relrowsecurity' => true, 'relforcerowsecurity' => true)
      expect(policies).to include('household_tenant_isolation')
    end
  end

  def expect_duplicate_open_period_rejected(source)
    attributes = {
      household: household,
      reason: 'side_effects',
      started_at: Time.current,
      recorded_by_membership: membership
    }.merge(source)
    described_class.create!(attributes)
    duplicate = described_class.new(attributes.merge(portable_id: SecureRandom.uuid))

    expect { duplicate.save!(validate: false) }.to raise_error(ActiveRecord::RecordNotUnique)
  end
end
