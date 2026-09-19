# frozen_string_literal: true

require 'rails_helper'
require Rails.root.join('db/migrate/20260919130000_retire_api_oidc_nonces')

RSpec.describe RetireApiOidcNonces do
  self.use_transactional_tests = false

  let(:connection) { ActiveRecord::Base.connection }
  let(:migration) { described_class.new }

  around do |example|
    connection.transaction(requires_new: true) do
      migrate_down
      example.run
      raise ActiveRecord::Rollback
    end
  ensure
    connection.schema_cache.clear!
  end

  it 'refuses to drop a nonce table containing data' do
    connection.execute(<<~SQL.squish)
      INSERT INTO api_oidc_nonces (issuer, subject, nonce, used_at, created_at, updated_at)
      VALUES ('https://issuer.example', 'subject', 'nonce', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
    SQL

    expect { migrate_up }.to raise_error(ActiveRecord::MigrationError, /api_oidc_nonces contains 1 row/)
    expect(connection.table_exists?(:api_oidc_nonces)).to be(true)
  end

  it 'drops an empty nonce table' do
    migrate_up

    expect(connection.table_exists?(:api_oidc_nonces)).to be(false)
  end

  it 'restores the original columns and indexes on rollback' do
    expect(connection.columns(:api_oidc_nonces).to_h { |column| [column.name, [column.type, column.null]] }).to eq(
      'id' => [:integer, false],
      'issuer' => [:string, false],
      'subject' => [:string, false],
      'nonce' => [:string, false],
      'used_at' => [:datetime, false],
      'created_at' => [:datetime, false],
      'updated_at' => [:datetime, false]
    )
    expect(connection.indexes(:api_oidc_nonces).to_h { |index| [index.name, [index.columns, index.unique]] }).to eq(
      'index_api_oidc_nonces_on_issuer_and_subject_and_nonce' => [%w[issuer subject nonce], true],
      'index_api_oidc_nonces_on_used_at' => [%w[used_at], false]
    )
  end

  def migrate_up
    ActiveRecord::Migration.suppress_messages { migration.migrate(:up) }
  end

  def migrate_down
    return if connection.table_exists?(:api_oidc_nonces)

    ActiveRecord::Migration.suppress_messages { migration.migrate(:down) }
  end
end
