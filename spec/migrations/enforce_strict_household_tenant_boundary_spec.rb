# frozen_string_literal: true

require 'rails_helper'

load Rails.root.join('db/migrate/20260624091000_enforce_strict_household_tenant_boundary.rb') unless
  defined?(EnforceStrictHouseholdTenantBoundary)
load Rails.root.join('db/migrate/20260713130000_add_household_ownership_to_carer_relationships.rb') unless
  defined?(AddHouseholdOwnershipToCarerRelationships)

RSpec.describe EnforceStrictHouseholdTenantBoundary do
  delegate :connection, to: :'ActiveRecord::Base'

  describe EnforceStrictHouseholdTenantBoundary do
    around do |example|
      connection.transaction(requires_new: true) do
        example.run
        raise ActiveRecord::Rollback
      end
    end

    it 'relaxes forced RLS before legacy household backfill runs' do
      migration = described_class.new
      table_name = 'people'

      connection.execute("ALTER TABLE #{connection.quote_table_name(table_name)} FORCE ROW LEVEL SECURITY")

      expect { migration.send(:relax_forced_rls_for_backfill) }
        .to change { forced_rls?(table_name) }
        .from(true)
        .to(false)
    end

    def forced_rls?(table_name)
      connection.select_value(<<~SQL.squish)
        SELECT relforcerowsecurity
        FROM pg_class
        WHERE oid = #{connection.quote(table_name)}::regclass
      SQL
    end
  end

  describe AddHouseholdOwnershipToCarerRelationships do
    describe 'legacy data verification' do
      it 'reads valid endpoints while migrating as the forced-RLS table owner' do
        migration = described_class.new

        connection.execute('SET LOCAL ROLE med_tracker_owner')
        connection.execute("SELECT set_config('med_tracker.current_household_id', '', true)")

        migration.send(:with_people_rls_relaxed) do
          expect(migration.send(:legacy_relationship_mismatches)).to be_empty
        end
      end

      it 'restores forced people RLS when legacy verification fails' do
        migration = described_class.new

        expect do
          migration.send(:with_people_rls_relaxed) { raise ActiveRecord::MigrationError }
        end.to raise_error(ActiveRecord::MigrationError)

        forced = connection.select_value(<<~SQL.squish)
          SELECT relforcerowsecurity
          FROM pg_class
          WHERE oid = 'people'::regclass
        SQL
        expect(forced).to be(true)
      end

      it 'aborts with the invalid relationship ids before backfilling' do
        migration = described_class.new
        allow(migration).to receive(:legacy_relationship_mismatches).and_return(
          [
            { 'id' => 41, 'patient_household_id' => nil, 'carer_household_id' => 7 },
            { 'id' => 42, 'patient_household_id' => 7, 'carer_household_id' => 8 }
          ]
        )

        expect { migration.send(:verify_legacy_relationships!) }
          .to raise_error(ActiveRecord::MigrationError, /41, 42/)
      end

      it 'allows a backfill only when every endpoint belongs to one household' do
        migration = described_class.new
        allow(migration).to receive(:legacy_relationship_mismatches).and_return([])

        expect { migration.send(:verify_legacy_relationships!) }.not_to raise_error
      end
    end

    describe 'database contract' do
      it 'requires household ownership and tenant-scoped uniqueness' do
        household_column = connection.columns(:carer_relationships).find { it.name == 'household_id' }
        indexes = connection.indexes(:carer_relationships).index_by(&:name)

        expect(household_column).not_to be_nil
        expect(household_column.null).to be(false)
        expect(indexes.fetch('index_carer_relationships_on_id_and_household_id').columns)
          .to eq(%w[id household_id])
        expect(indexes.fetch('index_carer_relationships_on_household_carer_patient').columns)
          .to eq(%w[household_id carer_id patient_id])
        expect(indexes.fetch('index_carer_relationships_on_household_carer_patient').unique).to be(true)
      end

      it 'binds both endpoints to the relationship household' do
        foreign_keys = connection.foreign_keys(:carer_relationships).index_by(&:name)

        expect(foreign_keys.fetch('fk_carer_relationships_household').to_table).to eq('households')
        expect(foreign_keys.fetch('fk_carer_relationships_carer_household').options).to include(
          column: %w[carer_id household_id],
          primary_key: %w[id household_id]
        )
        expect(foreign_keys.fetch('fk_carer_relationships_patient_household').options).to include(
          column: %w[patient_id household_id],
          primary_key: %w[id household_id]
        )
      end

      it 'forces the strict household tenant policy' do
        relation = connection.select_one(<<~SQL.squish)
          SELECT relrowsecurity, relforcerowsecurity
          FROM pg_class
          WHERE oid = 'carer_relationships'::regclass
        SQL
        policy = connection.select_one(<<~SQL.squish)
          SELECT qual, with_check
          FROM pg_policies
          WHERE schemaname = 'public'
            AND tablename = 'carer_relationships'
            AND policyname = 'household_tenant_isolation'
        SQL

        expect(relation).to eq('relrowsecurity' => true, 'relforcerowsecurity' => true)
        expect(policy.fetch('qual')).to include('med_tracker.current_household_id()')
        expect(policy.fetch('with_check')).to include('med_tracker.current_household_id()')
      end
    end
  end
end
