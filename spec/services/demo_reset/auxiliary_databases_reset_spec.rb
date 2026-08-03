# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DemoReset::AuxiliaryDatabasesReset do
  let(:configuration_class) { Data.define(:name) }

  it 'removes every runtime table from every configured auxiliary database', :aggregate_failures do
    queue = auxiliary_connection(%w[solid_queue_jobs solid_queue_processes schema_migrations])
    cache = auxiliary_connection(%w[solid_cache_entries ar_internal_metadata])
    configurations = %w[queue cache].map { |name| configuration_class.new(name:) }

    result = described_class.new(
      configurations:,
      connection_provider: provider_for('queue' => queue, 'cache' => cache)
    ).call

    expect(result).to eq(queue: 2, cache: 1)
    expect(queue).to have_received(:execute)
      .with('TRUNCATE TABLE "solid_queue_jobs", "solid_queue_processes" RESTART IDENTITY CASCADE')
    expect(cache).to have_received(:execute)
      .with('TRUNCATE TABLE "solid_cache_entries" RESTART IDENTITY CASCADE')
  end

  it 'is idempotent when an auxiliary database has no runtime rows' do
    connection = auxiliary_connection(%w[solid_queue_jobs schema_migrations])
    reset = described_class.new(
      configurations: [configuration_class.new(name: 'queue')],
      connection_provider: provider_for('queue' => connection)
    )

    expect(reset.call).to eq(queue: 1)
    expect(reset.call).to eq(queue: 1)
  end

  def auxiliary_connection(tables)
    instance_double(
      ActiveRecord::ConnectionAdapters::PostgreSQLAdapter,
      tables:,
      quote_table_name: nil
    ).tap do |connection|
      allow(connection).to receive(:quote_table_name) { |table| %("#{table}") }
      allow(connection).to receive(:transaction).and_yield
      allow(connection).to receive(:execute)
    end
  end

  def provider_for(connections)
    lambda do |configuration, &block|
      block.call(connections.fetch(configuration.name))
    end
  end
end
