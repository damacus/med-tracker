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

  def administrator_membership(household)
    household.household_memberships.create!(
      account: account("portable-import-administrator-#{SecureRandom.hex(4)}@example.test"),
      role: :administrator,
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

  def member_membership(household, person:)
    household.household_memberships.create!(
      account: account("portable-import-member-#{SecureRandom.hex(4)}@example.test"),
      person: person,
      role: :member,
      status: :active
    )
  end

  def grant_manage_access(household:, membership:, person:)
    PersonAccessGrant.create!(
      household: household,
      household_membership: membership,
      person: person,
      access_level: :manage,
      relationship_type: :self,
      granted_by_membership: membership
    )
  end

  def payload_with_unmanaged_schedule_reference(household:, manageable_person:, unmanaged_person:)
    medication = create(:medication, household: household, portable_id: 'existing-medication-portable')
    payload = solo_person_payload_for(manageable_person)
    payload[:records][:schedules] = [unmanaged_schedule_row(unmanaged_person, medication)]
    payload
  end

  def solo_person_payload_for(person)
    payload = portable_payload.deep_dup
    empty_record_types.each { |record_type| payload[:records][record_type] = [] }
    payload[:records][:people] = [
      payload[:records][:people].first.merge(portable_id: person.portable_id, location_portable_ids: [])
    ]
    payload
  end

  def empty_record_types
    %i[
      locations medications dosage_options schedules person_medications medication_takes notification_preferences
    ]
  end

  def unmanaged_schedule_row(unmanaged_person, medication)
    {
      portable_id: 'unmanaged-schedule-portable',
      person_portable_id: unmanaged_person.portable_id,
      medication_portable_id: medication.portable_id,
      dose_amount: 1,
      dose_unit: 'tablet',
      schedule_type: 'daily',
      start_date: '2026-01-01',
      end_date: '2026-12-31'
    }
  end

  def medication_update_payload_for(person:, medication:, current_supply:)
    payload = solo_person_payload_for(person)
    payload[:records][:medications] = [
      {
        portable_id: medication.portable_id,
        location_portable_id: medication.location.portable_id,
        name: medication.name,
        dose_amount: medication.dose_amount,
        dose_unit: medication.dose_unit,
        current_supply: current_supply,
        reorder_threshold: 2
      }
    ]
    payload
  end

  def dosage_update_payload_for(person:, dosage:, amount:)
    payload = solo_person_payload_for(person)
    payload[:records][:dosage_options] = [dosage_update_row(dosage, amount)]
    payload
  end

  def dosage_update_row(dosage, amount)
    {
      portable_id: dosage.portable_id,
      medication_portable_id: dosage.medication.portable_id,
      amount: amount,
      unit: dosage.unit,
      frequency: dosage.frequency,
      default_max_daily_doses: dosage.default_max_daily_doses,
      default_min_hours_between_doses: dosage.default_min_hours_between_doses,
      default_dose_cycle: dosage.default_dose_cycle,
      current_supply: 7,
      reorder_threshold: 1
    }
  end

  def portable_record_counts
    [Person, Location, Medication, MedicationDosageOption, Schedule, PersonMedication, MedicationTake,
     NotificationPreference].index_with(&:count)
  end

  def manager_location_payload(location:, name:, description:)
    payload = portable_payload.deep_dup
    payload[:records] = payload[:records].transform_values { [] }
    payload[:records][:locations] = [
      { portable_id: location.portable_id, name: name, description: description }
    ]
    payload
  end

  def expect_manager_location_import_allowed(membership, label)
    location = create(:location, household: membership.household, portable_id: "#{label.downcase}-location-portable")
    payload = manager_location_payload(location: location, name: "#{label} Updated Location",
                                       description: "#{label} description")
    result = import_result(household: membership.household, membership: membership, payload: payload, dry_run: false)

    expect(result).to be_applied
    expect(location.reload).to have_attributes(name: "#{label} Updated Location",
                                               description: "#{label} description")
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

  it 'grants the importing membership manage access to imported people' do
    household = create(:household)
    membership = owner_membership(household)

    result = import_result(household: household, membership: membership, dry_run: false)

    imported_person = Person.find_by!(household: household, portable_id: 'person-portable-1')
    expect(result).to be_applied
    expect(
      PersonAccessGrant.active.exists?(
        household: household,
        household_membership: membership,
        person: imported_person,
        access_level: :manage
      )
    ).to be(true)
  end

  it 'rejects member imports that reference unmanaged household people outside the top-level people array' do
    household = create(:household)
    manageable_person = create(:person, household: household, portable_id: 'manageable-person-portable')
    unmanaged_person = create(:person, household: household, portable_id: 'unmanaged-person-portable')
    membership = member_membership(household, person: manageable_person)
    grant_manage_access(household: household, membership: membership, person: manageable_person)
    payload = payload_with_unmanaged_schedule_reference(
      household: household,
      manageable_person: manageable_person,
      unmanaged_person: unmanaged_person
    )

    expect do
      import_result(household: household, membership: membership, payload: payload, dry_run: false)
    end.to raise_error(Pundit::NotAuthorizedError)

    expect(Schedule.exists?(household: household, portable_id: 'unmanaged-schedule-portable')).to be(false)
  end

  it 'rejects member imports that would create a new person without an existing manage grant' do
    household = create(:household)
    manageable_person = create(:person, household: household, portable_id: 'manageable-person-portable')
    membership = member_membership(household, person: manageable_person)
    grant_manage_access(household: household, membership: membership, person: manageable_person)

    expect do
      import_result(household: household, membership: membership, dry_run: false)
    end.to raise_error(Pundit::NotAuthorizedError)

    expect(Person.exists?(household: household, portable_id: 'person-portable-1')).to be(false)
  end

  it 'rejects imports without a household membership context' do
    household = create(:household)

    expect do
      import_result(household: household, membership: nil, dry_run: false)
    end.to raise_error(Pundit::NotAuthorizedError)
  end

  it 'allows member imports when every referenced person is already manageable' do
    household = create(:household)
    manageable_person = create(:person, household: household, portable_id: 'manageable-person-portable')
    membership = member_membership(household, person: manageable_person)
    grant_manage_access(household: household, membership: membership, person: manageable_person)
    payload = solo_person_payload_for(manageable_person)
    payload[:records][:people].first[:location_portable_ids] = []

    result = import_result(household: household, membership: membership, payload: payload)

    expect(result).not_to be_applied
    expect(result.errors).to be_empty
  end

  it 'rejects member location writes before the import writer mutates records' do
    household = create(:household)
    manageable_person = create(:person, household: household, portable_id: 'manageable-person-portable')
    location = create(:location, household: household, portable_id: 'location-portable', name: 'Original Shelf')
    original_description = location.description
    membership = member_membership(household, person: manageable_person)
    grant_manage_access(household: household, membership: membership, person: manageable_person)
    payload = solo_person_payload_for(manageable_person)
    payload[:records][:locations] = [
      { portable_id: location.portable_id, name: 'Delegated Rename', description: 'Changed by import' }
    ]

    expect do
      import_result(household: household, membership: membership, payload: payload, dry_run: false)
    end.to raise_error(Pundit::NotAuthorizedError)

    expect(location.reload).to have_attributes(name: 'Original Shelf', description: original_description)
  end

  it 'rejects member medication writes for medications outside granted people' do
    household = create(:household)
    manageable_person = create(:person, household: household, portable_id: 'manageable-person-portable')
    unmanaged_person = create(:person, household: household, portable_id: 'unmanaged-person-portable')
    medication = create(:medication, household: household, current_supply: 20)
    create(:person_medication, household: household, person: unmanaged_person, medication: medication)
    membership = member_membership(household, person: manageable_person)
    grant_manage_access(household: household, membership: membership, person: manageable_person)
    payload = medication_update_payload_for(person: manageable_person, medication: medication, current_supply: 99)

    expect do
      import_result(household: household, membership: membership, payload: payload, dry_run: false)
    end.to raise_error(Pundit::NotAuthorizedError)

    expect(medication.reload.current_supply).to eq(20)
  end

  it 'allows member medication writes for medications linked to granted people' do
    household = create(:household)
    manageable_person = create(:person, household: household, portable_id: 'manageable-person-portable')
    medication = create(:medication, household: household, current_supply: 20)
    create(:person_medication, household: household, person: manageable_person, medication: medication)
    membership = member_membership(household, person: manageable_person)
    grant_manage_access(household: household, membership: membership, person: manageable_person)
    payload = medication_update_payload_for(person: manageable_person, medication: medication, current_supply: 33)

    result = import_result(household: household, membership: membership, payload: payload, dry_run: false)

    expect(result).to be_applied
    expect(medication.reload.current_supply).to eq(33)
  end

  it 'rejects member dosage option writes for medications outside granted people' do
    household = create(:household)
    manageable_person = create(:person, household: household, portable_id: 'manageable-person-portable')
    unmanaged_person = create(:person, household: household, portable_id: 'unmanaged-person-portable')
    medication = create(:medication, household: household)
    create(:person_medication, household: household, person: unmanaged_person, medication: medication)
    dosage = create(:dosage, medication: medication, amount: 5)
    membership = member_membership(household, person: manageable_person)
    grant_manage_access(household: household, membership: membership, person: manageable_person)
    payload = dosage_update_payload_for(person: manageable_person, dosage: dosage, amount: 10)

    expect do
      import_result(household: household, membership: membership, payload: payload, dry_run: false)
    end.to raise_error(Pundit::NotAuthorizedError)

    expect(dosage.reload.amount).to eq(5)
  end

  it 'allows member dosage option writes for medications linked to granted people' do
    household = create(:household)
    manageable_person = create(:person, household: household, portable_id: 'manageable-person-portable')
    medication = create(:medication, household: household)
    create(:person_medication, household: household, person: manageable_person, medication: medication)
    dosage = create(:dosage, medication: medication, amount: 5)
    membership = member_membership(household, person: manageable_person)
    grant_manage_access(household: household, membership: membership, person: manageable_person)
    payload = dosage_update_payload_for(person: manageable_person, dosage: dosage, amount: 10)

    result = import_result(household: household, membership: membership, payload: payload, dry_run: false)

    expect(result).to be_applied
    expect(dosage.reload.amount).to eq(10)
  end

  it 'keeps owner location imports unrestricted within the household' do
    household = create(:household)
    expect_manager_location_import_allowed(owner_membership(household), 'Owner')
  end

  it 'keeps administrator location imports unrestricted within the household' do
    household = create(:household)
    expect_manager_location_import_allowed(administrator_membership(household), 'Administrator')
  end

  it 'keeps owner and administrator medication imports unrestricted within the household' do
    household = create(:household)
    unmanaged_person = create(:person, household: household, portable_id: 'unmanaged-person-portable')

    [owner_membership(household), administrator_membership(household)].each_with_index do |membership, index|
      medication = create(:medication, household: household, current_supply: 20 + index)
      create(:person_medication, household: household, person: unmanaged_person, medication: medication)
      payload = medication_update_payload_for(
        person: unmanaged_person,
        medication: medication,
        current_supply: 80 + index
      )

      result = import_result(household: household, membership: membership, payload: payload, dry_run: false)

      expect(result).to be_applied
      expect(medication.reload.current_supply).to eq(80 + index)
    end
  end

  it 'derives person access from person-medication medication take sources' do
    household = create(:household)
    membership = owner_membership(household)
    payload = portable_payload.deep_dup
    payload[:records][:schedules] = []
    payload[:records][:medication_takes].first.merge!(
      source_type: 'person_medication',
      source_portable_id: 'person-medication-portable-1'
    )

    result = import_result(household: household, membership: membership, payload: payload, dry_run: false)

    expect(result).to be_applied
    expect(MedicationTake.find_by!(portable_id: 'take-portable-1').person_medication.portable_id)
      .to eq('person-medication-portable-1')
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

  it 'rejects bundles with unsupported portable data formats' do
    household = create(:household)
    membership = owner_membership(household)
    payload = portable_payload.deep_dup
    payload[:format] = 'medtracker.portable.v0'

    expect do
      import_result(household: household, membership: membership, payload: payload, dry_run: false)
    end.to raise_error(described_class::Error, 'Unsupported portable data format')
  end

  it 'rejects bundles without record collections' do
    household = create(:household)
    membership = owner_membership(household)
    payload = portable_payload.except(:records)

    expect do
      import_result(household: household, membership: membership, payload: payload, dry_run: false)
    end.to raise_error(described_class::Error, 'Portable data records are required')
  end

  it 'rejects unsupported record collection names' do
    household = create(:household)
    membership = owner_membership(household)
    payload = portable_payload.deep_dup
    payload[:records][:appointments] = [{ portable_id: 'appointment-portable-1' }]

    expect do
      import_result(household: household, membership: membership, payload: payload, dry_run: false)
    end.to raise_error(described_class::Error, 'Unsupported portable record types: appointments')
  end

  it 'rejects record collections that are not arrays' do
    household = create(:household)
    membership = owner_membership(household)
    payload = portable_payload.deep_dup
    payload[:records][:people] = { portable_id: 'person-portable-1' }

    expect do
      import_result(household: household, membership: membership, payload: payload, dry_run: false)
    end.to raise_error(described_class::Error, 'people must be an array')
  end

  it 'rejects record rows without portable IDs' do
    household = create(:household)
    membership = owner_membership(household)
    payload = portable_payload.deep_dup
    payload[:records][:people].first.delete(:portable_id)

    expect do
      import_result(household: household, membership: membership, payload: payload, dry_run: false)
    end.to raise_error(described_class::Error, 'people[0].portable_id is required')
  end

  it 'rejects dependent people whose bundle marks them as having capacity' do
    household = create(:household)
    membership = owner_membership(household)
    payload = portable_payload.deep_dup
    payload[:records][:people].first[:person_type] = 'minor'
    payload[:records][:people].first[:date_of_birth] = 10.years.ago.to_date.iso8601
    payload[:records][:people].first[:has_capacity] = true

    result = import_result(household: household, membership: membership, payload: payload, dry_run: false)

    expect(result).not_to be_applied
    expect(result.errors.join).to include('has_capacity must be false')
    expect(Person.exists?(portable_id: 'person-portable-1')).to be(false)
  end

  it 'rejects malformed record rows before applying data' do
    household = create(:household)
    membership = owner_membership(household)
    payload = portable_payload.deep_dup
    payload[:records][:people] = ['not-a-record']

    expect do
      import_result(household: household, membership: membership, payload: payload, dry_run: false)
    end.to raise_error(PortableData::Importer::Error, /people must contain objects/)
  end

  it 'preserves person medication ordering during import' do
    household = create(:household)
    membership = owner_membership(household)
    payload = portable_payload.deep_dup
    payload[:records][:person_medications].first[:position] = 7

    result = import_result(household: household, membership: membership, payload: payload, dry_run: false)

    expect(result).to be_applied
    expect(PersonMedication.find_by!(portable_id: 'person-medication-portable-1').position).to eq(7)
  end

  it 'does not clear existing person medication ordering when an import row has a nil position' do
    household = create(:household)
    membership = owner_membership(household)
    payload = portable_payload.deep_dup
    payload[:records][:person_medications].first[:position] = 7
    import_result(household: household, membership: membership, payload: payload, dry_run: false)

    payload[:records][:person_medications].first[:position] = nil
    result = import_result(household: household, membership: membership, payload: payload, dry_run: false)

    expect(result).to be_applied
    expect(PersonMedication.find_by!(portable_id: 'person-medication-portable-1').position).to eq(7)
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
