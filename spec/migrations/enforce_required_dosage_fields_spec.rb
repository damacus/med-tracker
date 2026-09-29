require 'rails_helper'

load Rails.root.join('db/migrate/20260926000000_enforce_required_dosage_fields.rb') unless
  defined?(EnforceRequiredDosageFields)

RSpec.describe EnforceRequiredDosageFields do
  fixtures :all

  let(:connection) { ActiveRecord::Base.connection }
  let(:required_fields) do
    %w[amount unit frequency default_max_daily_doses default_min_hours_between_doses default_dose_cycle]
  end
  let(:dosage) { dosages(:paracetamol_adult) }

  it 'marks every required dosage field NOT NULL and preserves valid rows' do
    original = dosage.attributes.slice(*required_fields)

    required_fields.each do |field|
      expect(connection.columns(:dosages).find { |column| column.name == field }.null).to be(false)
      expect do
        connection.transaction(requires_new: true) { inject_null(field) }
      end.to raise_error(ActiveRecord::NotNullViolation)
    end

    expect(dosage.reload.attributes.slice(*required_fields)).to eq(original)
  end

  it 'refuses existing NULL values in each required field without replacing or deleting the row' do
    required_fields.each { |field| expect_migration_to_reject_null(field) }
  end

  def inject_null(field)
    connection.exec_update("UPDATE dosages SET #{connection.quote_column_name(field)} = NULL WHERE id = #{dosage.id}")
  end

  def expect_migration_to_reject_null(field)
    connection.transaction(requires_new: true) do
      described_class.new.down
      inject_null(field)
      expect_migration_error(field)
      expect(dosage.reload.public_send(field)).to be_nil
      expect_nullable_columns
      raise ActiveRecord::Rollback
    end
  end

  def expect_migration_error(field)
    expect do
      connection.transaction(requires_new: true) { described_class.new.up }
    end.to raise_error(ActiveRecord::MigrationError, /dosages\.#{field}: existing NULL values/)
  end

  def expect_nullable_columns
    required_fields.each do |field|
      expect(connection.columns(:dosages).find { |column| column.name == field }.null).to be(true)
    end
  end
end
