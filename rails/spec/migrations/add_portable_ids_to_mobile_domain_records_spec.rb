# frozen_string_literal: true

require 'rails_helper'

load Rails.root.join('db/migrate/20260705120000_add_portable_ids_to_mobile_domain_records.rb') unless
  defined?(AddPortableIdsToMobileDomainRecords)

RSpec.describe AddPortableIdsToMobileDomainRecords do
  delegate :connection, to: :'ActiveRecord::Base'

  it 'backfills every populated mobile-domain table through forced row-level security' do
    allow_null_portable_ids(described_class::TABLES)
    records = mobile_domain_records
    clear_portable_ids(records)
    connection.execute('SET CONSTRAINTS ALL IMMEDIATE')

    with_owner_role do
      records.each_key { |table_name| described_class.new.send(:prepare_portable_ids, table_name) }
    end

    expect(portable_ids(records)).to all(match(/\A[0-9a-f-]{36}\z/))
  end

  it 'preserves existing portable ids when the backfill is retried' do
    records = mobile_domain_records
    original_ids = portable_ids(records)
    connection.execute('SET CONSTRAINTS ALL IMMEDIATE')

    with_owner_role do
      records.each_key { |table_name| described_class.new.send(:prepare_portable_ids, table_name) }
    end

    expect(portable_ids(records)).to eq(original_ids)
  end

  it 'finishes a partially applied table without replacing its existing portable ids' do
    table_name = :portable_id_migration_records
    existing_portable_id = SecureRandom.uuid
    create_portable_id_table(table_name)
    insert_portable_ids(table_name, existing_portable_id, nil, nil)
    stub_const('AddPortableIdsToMobileDomainRecords::TABLES', [table_name].freeze)

    described_class.new.up

    expect_portable_ids_backfilled(table_name, existing_portable_id)
    expect_portable_id_schema_enforced(table_name)
  end

  it 'reports remaining null rows before applying the not-null constraint' do
    table_name = :portable_id_migration_failures
    create_portable_id_table(table_name)
    insert_portable_ids(table_name, nil)
    stub_const('AddPortableIdsToMobileDomainRecords::TABLES', [table_name].freeze)
    migration = described_class.new
    allow(migration).to receive(:backfill_portable_ids)

    expect { migration.up }
      .to raise_error(
        ActiveRecord::IrreversibleMigration,
        /portable_id_migration_failures has 1 rows without portable_id/
      )
  end

  def mobile_domain_records
    household = create(:household)
    records = base_mobile_domain_records(household)
    dosage = create(:dosage, household: household, medication: records.fetch(:medications))
    records[:dosages] = dosage

    add_dependent_mobile_domain_records(records, household, dosage)
  end

  def base_mobile_domain_records(household)
    person = create(:person, household: household)
    location = create(:location, household: household)
    medication = create(:medication, household: household, location: location)

    { people: person, locations: location, medications: medication }
  end

  def create_schedule(household, person, medication, dosage)
    create(
      :schedule,
      household: household,
      person: person,
      medication: medication,
      dosage: dosage
    )
  end

  def create_person_medication(household, person, medication, dosage)
    create(
      :person_medication,
      household: household,
      person: person,
      medication: medication,
      dosage: dosage
    )
  end

  def add_dependent_mobile_domain_records(records, household, dosage)
    person = records.fetch(:people)
    medication = records.fetch(:medications)
    schedule = create_schedule(household, person, medication, dosage)
    records[:schedules] = schedule
    records[:person_medications] = create_person_medication(household, person, medication, dosage)
    records[:medication_takes] = create(:medication_take, :for_schedule, household: household, schedule: schedule)
    records[:notification_preferences] = create(:notification_preference, household: household, person: person)
    records
  end

  def create_portable_id_table(table_name)
    connection.create_table(table_name) do |table|
      table.bigint :household_id, null: false
      table.string :portable_id
    end
  end

  def insert_portable_ids(table_name, *portable_ids)
    values = portable_ids.map { |portable_id| "(1, #{connection.quote(portable_id)})" }.join(', ')
    connection.execute(<<~SQL.squish)
      INSERT INTO #{connection.quote_table_name(table_name)} (household_id, portable_id)
      VALUES #{values}
    SQL
  end

  def expect_portable_ids_backfilled(table_name, existing_portable_id)
    ids = connection.select_values(
      "SELECT portable_id FROM #{connection.quote_table_name(table_name)} ORDER BY id"
    )
    uuid = match(/\A[0-9a-f-]{36}\z/)
    expect(ids).to match([existing_portable_id, uuid, uuid])
    expect(ids.uniq.size).to eq(3)
  end

  def expect_portable_id_schema_enforced(table_name)
    column = connection.columns(table_name).find { |candidate| candidate.name == 'portable_id' }
    expect(column.null).to be(false)
    expect(connection.index_exists?(table_name, %i[household_id portable_id], unique: true)).to be(true)
  end

  def clear_portable_ids(records)
    records.each do |table_name, record|
      connection.execute(<<~SQL.squish)
        UPDATE #{connection.quote_table_name(table_name)}
        SET portable_id = NULL
        WHERE id = #{connection.quote(record.id)}
      SQL
    end
  end

  def allow_null_portable_ids(table_names)
    table_names.each { |table_name| connection.change_column_null(table_name, :portable_id, true) }
  end

  def portable_ids(records)
    records.map do |table_name, record|
      connection.select_value(<<~SQL.squish)
        SELECT portable_id
        FROM #{connection.quote_table_name(table_name)}
        WHERE id = #{connection.quote(record.id)}
      SQL
    end
  end

  def with_owner_role
    connection.execute('SET LOCAL ROLE med_tracker_owner')
    yield
  ensure
    connection.execute('RESET ROLE')
  end
end
