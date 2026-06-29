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
    household = Household.create!(name: "#{person_name} Household")
    person = Person.create!(
      household: household,
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
    household = Household.create!(name: "#{name} Household")
    Person.create!(
      household: household,
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
