# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Households::LifecycleCutoffLock do
  it 'uses the established household purge namespace' do
    connection = instance_double(ActiveRecord::ConnectionAdapters::PostgreSQLAdapter)
    allow(ActiveRecord::Base).to receive(:connection).and_return(connection)
    allow(connection).to receive(:select_value).and_return(1, true)

    described_class.with(household_id: 41) { nil }

    expect(connection).to have_received(:select_value).with(include('med_tracker.household_purge:41')).twice
  end

  it 'releases the cutoff lock when the protected operation raises' do
    connection = instance_double(ActiveRecord::ConnectionAdapters::PostgreSQLAdapter)
    allow(ActiveRecord::Base).to receive(:connection).and_return(connection)
    allow(connection).to receive(:select_value).and_return(1, true)

    expect do
      described_class.with(household_id: 42) { raise 'cutoff failure' }
    end.to raise_error('cutoff failure')

    expect(connection).to have_received(:select_value).with(include('pg_advisory_unlock')).once
  end

  it 'balances nested same-household acquisitions when the inner operation raises' do
    connection = instance_double(ActiveRecord::ConnectionAdapters::PostgreSQLAdapter)
    allow(ActiveRecord::Base).to receive(:connection).and_return(connection)
    allow(connection).to receive(:select_value).and_return(1, true)

    expect do
      described_class.with(household_id: 43) do
        described_class.with(household_id: 43) { raise 'nested cutoff failure' }
      end
    end.to raise_error('nested cutoff failure')

    expect(connection).to have_received(:select_value).with(include('pg_advisory_lock')).twice
    expect(connection).to have_received(:select_value).with(include('pg_advisory_unlock')).twice
  end

  it 'does not make an unrelated household wait for the held household key' do
    first_key = 'med_tracker.household_purge:51'
    second_key = 'med_tracker.household_purge:52'
    first_connection, second_connection = pg_connections

    first_connection.exec_params(acquire_sql, [first_key])

    expect(second_connection.backend_pid).not_to eq(first_connection.backend_pid)
    expect(try_lock?(second_connection, second_key)).to be(true)
    expect(try_lock?(second_connection, first_key)).to be(false)
  ensure
    second_connection&.exec_params(unlock_sql, [second_key])
    first_connection&.exec_params(unlock_sql, [first_key])
    second_connection&.close
    first_connection&.close
  end

  def acquire_sql
    'WITH lock_acquired AS MATERIALIZED (' \
      'SELECT pg_advisory_lock(hashtextextended($1, 0))' \
      ') SELECT 1 FROM lock_acquired'
  end

  def try_lock_sql
    'SELECT pg_try_advisory_lock(hashtextextended($1, 0))'
  end

  def unlock_sql
    'SELECT pg_advisory_unlock(hashtextextended($1, 0))'
  end

  def pg_connections
    parameters = ActiveRecord::Base.connection.raw_connection.conninfo_hash.compact_blank
    [PG.connect(parameters), PG.connect(parameters)]
  end

  def try_lock?(connection, lock_key)
    connection.exec_params(try_lock_sql, [lock_key]).getvalue(0, 0) == 't'
  end
end
