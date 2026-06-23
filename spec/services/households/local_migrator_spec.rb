# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Households::LocalMigrator do
  let(:idempotent_backfill) do
    owner, owner_person = create_legacy_account(email: 'local-owner@example.test', person_name: 'Local Owner')
    doctor_account, doctor_person = create_legacy_account(email: 'local-doctor@example.test',
                                                          person_name: 'Local Doctor')
    patient = create_legacy_person('Local Patient')
    CarerRelationship.create!(carer: owner_person, patient: patient, relationship_type: 'parent', active: true)
    CarerRelationship.create!(carer: doctor_person, patient: patient, relationship_type: 'professional_carer',
                              active: true)
    location = Location.create!(name: 'Home')
    medication = Medication.create!(name: 'Paracetamol', location: location, household: nil, reorder_threshold: 10)
    dosage = create_dosage(medication)
    schedule = create_schedule(patient: patient, medication: medication, dosage: dosage)
    take = MedicationTake.create!(schedule: schedule, taken_at: Time.current, dose_amount: 5, dose_unit: 'ml')
    NotificationPreference.create!(person: patient)

    {
      owner: owner, owner_person: owner_person, doctor_account: doctor_account, doctor_person: doctor_person,
      patient: patient, medication: medication, dosage: dosage, schedule: schedule, take: take
    }
  end
  let(:edge_case_backfill) do
    owner, owner_person = create_legacy_account(email: 'free-owner@example.test', person_name: 'Free Owner')
    admin_account, admin_person = create_legacy_account(email: 'free-admin@example.test', person_name: 'Free Admin')
    orphan_account = Account.create!(email: 'orphan-member@example.test', status: :verified)
    patient = create_legacy_person('Free Patient')
    unlinked_carer = create_legacy_person('Unlinked Carer')
    keeper_location, duplicate_location = create_duplicate_locations(owner_person, patient)
    medication = Medication.create!(name: 'Ibuprofen', location: duplicate_location, household: nil,
                                    reorder_threshold: 5)
    CarerRelationship.create!(carer: admin_person, patient: patient, relationship_type: 'guardian', active: true)
    CarerRelationship.create!(carer: unlinked_carer, patient: patient, relationship_type: 'family_member',
                              active: true)

    {
      owner: owner, owner_person: owner_person, admin_account: admin_account, orphan_account: orphan_account,
      patient: patient,
      keeper_location: keeper_location, duplicate_location: duplicate_location, medication: medication
    }
  end

  def create_legacy_account(email:, person_name:)
    account = Account.create!(email: email, status: :verified)
    person = Person.create!(
      account: account,
      name: person_name,
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
    user = User.create!(person: person, email_address: email, active: true)
    [account, person, user]
  end

  def create_legacy_person(name)
    Person.create!(
      name: name,
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
  end

  def create_dosage(medication)
    medication.dosage_records.create!(
      amount: 5,
      unit: 'ml',
      frequency: 'Daily',
      default_max_daily_doses: 4,
      default_min_hours_between_doses: 4,
      default_dose_cycle: :daily
    )
  end

  def create_schedule(patient:, medication:, dosage:)
    Schedule.create!(
      person: patient,
      medication: medication,
      source_dosage_option: dosage,
      start_date: Time.zone.today,
      end_date: 1.week.from_now.to_date,
      dose_amount: 5,
      dose_unit: 'ml',
      frequency: 'Daily'
    )
  end

  def create_duplicate_locations(owner_person, patient)
    keeper_location = Location.create!(name: 'Clinic Duplicate')
    duplicate_location = Location.create!(name: 'clinic duplicate')
    LocationMembership.create!(location: keeper_location, person: owner_person)
    LocationMembership.create!(location: duplicate_location, person: owner_person)
    LocationMembership.create!(location: duplicate_location, person: patient)
    [keeper_location, duplicate_location]
  end

  def run_migration(owner, household_name)
    described_class.new(
      owner_email: owner.email,
      household_name: household_name,
      apply: true
    ).call
  end

  def idempotent_people
    idempotent_backfill.values_at(:owner_person, :doctor_person, :patient)
  end

  def idempotent_tenant_records
    [
      idempotent_backfill.fetch(:medication),
      idempotent_backfill.fetch(:dosage),
      idempotent_backfill.fetch(:schedule),
      NotificationPreference.sole
    ]
  end

  def expect_manage_grant(household, person)
    expect(household.person_access_grants.active.where(person: person, access_level: :manage).count).to eq(1)
  end

  def expect_record_grant(household, person)
    expect(household.person_access_grants.active.where(person: person, access_level: :record).count).to eq(1)
  end

  def stub_account_enumeration(*accounts)
    relation = instance_double(ActiveRecord::Relation)
    allow(relation).to receive(:find_each) { |&block| accounts.each(&block) }
    allow(Account).to receive(:includes).with(person: :user).and_return(relation)
  end

  def stub_legacy_role(user, role)
    allow(user).to receive(:has_attribute?).with(:role).and_return(true)
    allow(user).to receive(:read_attribute_before_type_cast).with(:role).and_return(role)
  end

  def stub_legacy_professional_role(account, person, user, role)
    allow(account).to receive(:person).and_return(person)
    allow(person).to receive(:user).and_return(user)
    stub_legacy_role(user, role)
  end

  def write_raw_column(record, column_name, value)
    connection = record.class.connection
    table_name = connection.quote_table_name(record.class.table_name)
    column = connection.quote_column_name(column_name)
    sql = record.class.sanitize_sql_array(["UPDATE #{table_name} SET #{column} = ? WHERE id = ?", value, record.id])
    connection.execute(sql)
    record[column_name] = value
  end

  it 'reads pre-cutover subscription plans and roles only when legacy columns are present' do
    migrator = described_class.new(owner_email: 'legacy@example.test', household_name: 'Legacy', apply: false)
    connection = ActiveRecord::Base.connection
    legacy_user = instance_double(User, has_attribute?: true)

    allow(connection).to receive(:column_exists?).and_call_original
    allow(connection).to receive(:column_exists?).with(:accounts, :subscription_plan).and_return(true)
    allow(Account).to receive(:exists?).with(subscription_plan: 'family_plus').and_return(true)
    allow(legacy_user).to receive(:read_attribute_before_type_cast).with(:role).and_return(1)

    expect(migrator.send(:highest_subscription_plan)).to eq(:family_plus)
    expect(migrator.send(:legacy_user_role, legacy_user)).to eq(:doctor)
  end

  it 'dry-runs without mutating data and reports counts' do
    owner, = create_legacy_account(
      email: 'dry-run-owner@example.test',
      person_name: 'Dry Owner'
    )

    result = described_class.new(
      owner_email: owner.email,
      household_name: 'Dry Run Household',
      apply: false
    ).call

    expect(result).not_to be_applied
    expect(result.before_counts.fetch('accounts')).to be >= 1
    expect(result.before_counts.fetch('people')).to be >= 1
    expect(result.after_counts).to eq(result.before_counts)
    expect(Household.exists?(name: 'Dry Run Household')).to be(false)
  end

  it 'creates one household idempotently' do
    first = run_migration(idempotent_backfill.fetch(:owner), 'Local Household')
    second = run_migration(idempotent_backfill.fetch(:owner), 'Local Household')
    household = Household.find_by!(name: 'Local Household')

    expect(first).to be_applied
    expect(second).to be_applied
    expect(Household.where(name: 'Local Household').count).to eq(1)
    expect(household.subscription_plan).to eq('free')
  end

  it 'backfills memberships and people into the local household' do
    household = run_migration(idempotent_backfill.fetch(:owner), 'Local Household').household

    expect(household.household_memberships.count).to be >= 2
    expect(household.household_memberships.find_by!(account: idempotent_backfill.fetch(:owner)).role).to eq('owner')
    expect(household.household_memberships.find_by!(account: idempotent_backfill.fetch(:doctor_account)).role)
      .to eq('member')
    expect(idempotent_backfill.fetch(:doctor_person).reload.professional_title).to be_nil
    expect(idempotent_people.map { |person| person.reload.household }).to all(eq(household))
  end

  it 'backfills tenant-owned medication records into the local household' do
    household = run_migration(idempotent_backfill.fetch(:owner), 'Local Household').household
    surviving_location = idempotent_backfill.fetch(:medication).reload.location

    expect(surviving_location.household).to eq(household)
    expect(idempotent_tenant_records.map { |record| record.reload.household })
      .to all(eq(household))
    expect(idempotent_backfill.fetch(:take).reload.household).to eq(household)
    expect(household.locations.where(name: 'Home').count).to eq(1)
  end

  it 'backfills relationship grants into the local household' do
    household = run_migration(idempotent_backfill.fetch(:owner), 'Local Household').household

    expect_manage_grant(household, idempotent_backfill.fetch(:owner_person))
    expect_manage_grant(household, idempotent_backfill.fetch(:doctor_person))
    expect_manage_grant(household, idempotent_backfill.fetch(:patient))
    expect_record_grant(household, idempotent_backfill.fetch(:patient))
  end

  it 'backfills free household membership edge cases' do
    household = run_migration(edge_case_backfill.fetch(:owner), 'Free Local Household').household

    expect(household.subscription_plan).to eq('free')
    expect(household.household_memberships.find_by!(account: edge_case_backfill.fetch(:admin_account))).to be_member
    expect(household.household_memberships.find_by!(account: edge_case_backfill.fetch(:orphan_account)).person)
      .to be_nil
  end

  it 'deduplicates free household locations during backfill' do
    household = run_migration(edge_case_backfill.fetch(:owner), 'Free Local Household').household
    keeper_location = edge_case_backfill.fetch(:keeper_location)

    expect(Location.where(id: edge_case_backfill.fetch(:duplicate_location).id)).not_to exist
    expect(edge_case_backfill.fetch(:medication).reload.location).to eq(keeper_location)
    expect(keeper_location.reload.household).to eq(household)
    expect(LocationMembership.where(location: keeper_location, person: edge_case_backfill.fetch(:owner_person)).count)
      .to eq(1)
  end

  it 'backfills free household relationship grants' do
    household = run_migration(edge_case_backfill.fetch(:owner), 'Free Local Household').household

    expect(LocationMembership.find_by!(location: edge_case_backfill.fetch(:keeper_location),
                                       person: edge_case_backfill.fetch(:patient)).household).to eq(household)
    expect(
      household.person_access_grants.active.find_by!(
        household_membership: household.household_memberships.find_by!(
          account: edge_case_backfill.fetch(:admin_account)
        ),
        person: edge_case_backfill.fetch(:patient)
      )
    ).to have_attributes(access_level: 'record', relationship_type: 'family_member')
  end

  it 'backfills duplicate-location inventory rows without validating legacy invalid medication data' do
    owner, owner_person = create_legacy_account(email: 'invalid-location-owner@example.test',
                                                person_name: 'Invalid Location Owner')
    patient = create_legacy_person('Invalid Location Patient')
    keeper_location, duplicate_location = create_duplicate_locations(owner_person, patient)
    medication = Medication.create!(name: 'Legacy Invalid Inventory', location: duplicate_location,
                                    household: nil, reorder_threshold: 5)
    write_raw_column(medication, :barcode, 'legacy-invalid-barcode')

    household = run_migration(owner, 'Invalid Location Household').household

    expect(medication.reload).to have_attributes(location: keeper_location, household: household)
    expect(medication.barcode).to eq('legacy-invalid-barcode')
  end

  it 'migrates legacy doctor and nurse roles into professional titles' do
    owner, owner_person, = create_legacy_account(email: 'legacy-title-owner@example.test',
                                                 person_name: 'Legacy Title Owner')
    doctor_account, doctor_person, doctor_user = create_legacy_account(email: 'legacy-doctor@example.test',
                                                                       person_name: 'Legacy Doctor')
    nurse_account, nurse_person, nurse_user = create_legacy_account(email: 'legacy-nurse@example.test',
                                                                    person_name: 'Legacy Nurse')
    stub_account_enumeration(owner, doctor_account, nurse_account)
    allow(owner).to receive(:person).and_return(owner_person)
    stub_legacy_professional_role(doctor_account, doctor_person, doctor_user, 1)
    stub_legacy_professional_role(nurse_account, nurse_person, nurse_user, 2)

    household = run_migration(owner, 'Legacy Title Household').household

    expect(household.household_memberships.find_by!(account: doctor_account)).to be_member
    expect(household.household_memberships.find_by!(account: nurse_account)).to be_member
    expect(doctor_person.reload.professional_title).to eq('doctor')
    expect(nurse_person.reload.professional_title).to eq('nurse')
  end

  it 'fails unless exactly one verified owner account matches the email' do
    Account.create!(email: 'unverified-owner@example.test', status: :unverified)

    expect do
      described_class.new(
        owner_email: 'unverified-owner@example.test',
        household_name: 'Invalid Household',
        apply: true
      ).call
    end.to raise_error(described_class::Error, /exactly one verified owner account/)
  end

  it 'requires owner email and household name' do
    expect do
      described_class.new(owner_email: ' ', household_name: 'Household', apply: false).call
    end.to raise_error(described_class::Error, /OWNER_EMAIL/)

    expect do
      described_class.new(owner_email: 'owner@example.test', household_name: ' ', apply: false).call
    end.to raise_error(described_class::Error, /HOUSEHOLD_NAME/)
  end
end
