# frozen_string_literal: true

require 'spec_helper'
require 'active_record'
require_relative '../../db/migrate/20250930220454_fix_person_type_and_capacity_defaults'

RSpec.describe FixPersonTypeAndCapacityDefaults do
  subject(:migration) { instrumented_migration.new }

  let(:instrumented_migration) do
    Class.new(described_class) do
      attr_reader :executed_sql

      def initialize
        super
        @executed_sql = []
      end

      def execute(sql)
        executed_sql << sql
      end

      def change_column_default(*)
        # no-op to avoid touching real connection
      end

      def change_column_null(*)
        # no-op to avoid touching real connection
      end
    end
  end

  describe '#up' do
    it 'updates person_type using integer literal' do
      migration.up

      expect(migration.executed_sql).to include(a_string_matching(/SET person_type = 0/i))
    end

    it 'updates has_capacity using SQL boolean literal' do
      migration.up

      expect(migration.executed_sql).to include(a_string_matching(/SET has_capacity = TRUE/i))
    end
  end
end
