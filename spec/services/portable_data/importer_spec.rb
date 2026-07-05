# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PortableData::Importer do
  let(:passphrase) { 'correct horse battery staple' }
  let(:portable_payload) do
    {
      format: 'medtracker.portable.v1',
      scope: 'single_person',
      exported_at: Time.current.iso8601,
      source_instance_id: 'mobile:test',
      records: {
        people: [
          {
            portable_id: 'person-portable-1',
            name: 'Solo User',
            email: 'solo@example.test',
            date_of_birth: '1990-01-01',
            person_type: 'adult',
            has_capacity: true,
            location_portable_ids: ['location-portable-1']
          }
        ],
        locations: [
          { portable_id: 'location-portable-1', name: 'Home', description: 'Kitchen shelf' }
        ],
        medications: [
          {
            portable_id: 'medication-portable-1',
            location_portable_id: 'location-portable-1',
            name: 'Solo Medicine',
            dose_amount: 5,
            dose_unit: 'ml',
            current_supply: 12,
            reorder_threshold: 3
          }
        ],
        dosage_options: [
          {
            portable_id: 'dosage-portable-1',
            medication_portable_id: 'medication-portable-1',
            amount: 5,
            unit: 'ml',
            frequency: 'Daily',
            default_max_daily_doses: 2,
            default_min_hours_between_doses: 4,
            default_dose_cycle: 'daily'
          }
        ],
        schedules: [
          {
            portable_id: 'schedule-portable-1',
            person_portable_id: 'person-portable-1',
            medication_portable_id: 'medication-portable-1',
            source_dosage_option_portable_id: 'dosage-portable-1',
            dose_amount: 5,
            dose_unit: 'ml',
            frequency: 'Daily',
            dose_cycle: 'daily',
            schedule_type: 'daily',
            start_date: '2026-01-01',
            end_date: '2026-12-31',
            max_daily_doses: 2,
            min_hours_between_doses: 4,
            active: true
          }
        ],
        person_medications: [
          {
            portable_id: 'person-medication-portable-1',
            person_portable_id: 'person-portable-1',
            medication_portable_id: 'medication-portable-1',
            source_dosage_option_portable_id: 'dosage-portable-1',
            dose_amount: 5,
            dose_unit: 'ml',
            dose_cycle: 'daily',
            administration_kind: 'as_needed',
            active: true
          }
        ],
        medication_takes: [
          {
            portable_id: 'take-portable-1',
            client_uuid: 'take-client-uuid-1',
            source_type: 'schedule',
            source_portable_id: 'schedule-portable-1',
            taken_at: '2026-02-01T08:30:00Z',
            dose_amount: 5,
            dose_unit: 'ml'
          }
        ],
        notification_preferences: [
          {
            portable_id: 'preference-portable-1',
            person_portable_id: 'person-portable-1',
            enabled: true,
            dose_due_enabled: true,
            missed_dose_enabled: true,
            low_stock_enabled: false,
            private_text_enabled: true,
            morning_time: '08:00:00'
          }
        ]
      }
    }
  end
  let(:encrypted_payload) { PortableData::Encryptor.encrypt(portable_payload, passphrase: passphrase) }

  def account(email)
    Account.create!(email: email, status: :verified)
  end

  def owner_membership(household)
    household.household_memberships.create!(
      account: account('portable-import-owner@example.test'),
      role: :owner,
      status: :active
    )
  end

  def import_result(household:, membership:, payload: portable_payload, dry_run: true)
    described_class.new(
      household: household,
      membership: membership,
      envelope: PortableData::Encryptor.encrypt(payload, passphrase: passphrase),
      passphrase: passphrase,
      options: { dry_run: dry_run }
    ).call
  end

  def apply_import(household:, membership:, envelope: encrypted_payload)
    described_class.new(
      household: household,
      membership: membership,
      envelope: envelope,
      passphrase: passphrase,
      options: { dry_run: false }
    ).call
  end

  def portable_record_counts
    [Person, Location, Medication, MedicationDosageOption, Schedule, PersonMedication, MedicationTake,
     NotificationPreference].index_with(&:count)
  end

  def expect_import_creates_portable_graph(household:, membership:, envelope:)
    before_counts = portable_record_counts
    apply_import(household: household, membership: membership, envelope: envelope)
    expect(portable_record_counts).to eq(before_counts.transform_values { |count| count + 1 })
  end

  it 'dry-runs without writing records and returns record counts' do
    household = create(:household)
    membership = owner_membership(household)

    result = import_result(household: household, membership: membership)

    expect(result).not_to be_applied
    expect(result.counts).to include('people' => 1, 'medications' => 1, 'medication_takes' => 1)
    expect(Person.exists?(portable_id: 'person-portable-1')).to be(false)
  end

  it 'applies an encrypted portable bundle idempotently' do
    household = create(:household)
    membership = owner_membership(household)
    envelope = encrypted_payload

    expect_import_creates_portable_graph(household: household, membership: membership, envelope: envelope)

    expect do
      apply_import(household: household, membership: membership, envelope: envelope)
    end.not_to change(Person, :count)

    medication = Medication.find_by!(portable_id: 'medication-portable-1')
    take = MedicationTake.find_by!(portable_id: 'take-portable-1')
    expect(medication.current_supply).to eq(12)
    expect(take.schedule.portable_id).to eq('schedule-portable-1')
  end

  it 'rejects bundles that contain Rails numeric IDs' do
    household = create(:household)
    membership = owner_membership(household)
    payload = portable_payload.deep_dup
    payload[:records][:people].first[:id] = 123
    payload[:records][:schedules].first[:person_id] = 456

    result = import_result(household: household, membership: membership, payload: payload)

    expect(result).not_to be_applied
    expect(result.errors.join).to include('Rails numeric IDs')
    expect(Person.exists?(portable_id: 'person-portable-1')).to be(false)
  end

  it 'reports target conflicts before applying records' do
    household = create(:household)
    membership = owner_membership(household)
    existing_location = create(:location, household: household, name: 'Home')

    result = import_result(household: household, membership: membership)

    expect(result).not_to be_applied
    expect(result.conflicts).to include(
      include(
        record_type: 'locations',
        portable_id: 'location-portable-1',
        existing_portable_id: existing_location.portable_id
      )
    )
    expect(Location.exists?(household: household, portable_id: 'location-portable-1')).to be(false)
  end

  it 'does not write partial data when apply validation fails' do
    household = create(:household)
    membership = owner_membership(household)
    payload = portable_payload.deep_dup
    payload[:records][:medications].first[:name] = nil

    expect do
      result = import_result(household: household, membership: membership, payload: payload, dry_run: false)
      expect(result).not_to be_applied
      expect(result.errors.join).to include("Name can't be blank")
    end.not_to change(Person, :count)

    expect(Location.exists?(household: household, portable_id: 'location-portable-1')).to be(false)
  end

  it 'rejects bundles that cannot be decrypted' do
    household = create(:household)
    membership = owner_membership(household)

    expect do
      described_class.new(
        household: household,
        membership: membership,
        envelope: encrypted_payload,
        passphrase: 'wrong passphrase',
        options: { dry_run: true }
      ).call
    end.to raise_error(PortableData::Encryptor::Error)
  end
end
