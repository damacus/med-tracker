# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Seeds' do
  self.use_transactional_tests = false

  before { truncate_runtime_tables }

  around do |example|
    example.run
  ensure
    restore_fixture_baseline
  end

  it 'does not leave unmatched audit versions after seeding synthetic fixtures', :aggregate_failures do
    load Rails.root.join('db/seeds.rb')

    expect(PaperTrail::Version.count).to be_zero
    expect(verify_seeded_evidence).to be_valid
    expect(User.find_by!(email_address: 'damacus@example.com').authenticate('password')).to be_truthy
  end

  it 'keeps PaperTrail enabled for non-local seed paths' do
    allow(Rails).to receive(:env).and_return(ActiveSupport::StringInquirer.new('staging'))

    load Rails.root.join('db/seeds.rb')

    expect(PaperTrail::Version.where(item_type: 'Location')).to exist
  end

  def verify_seeded_evidence
    connection = ActiveRecord::Base.connection
    connection.execute('SET ROLE med_tracker_audit_verifier')
    Audit::Verification::DatabaseVerifier.new.call
  ensure
    connection.execute('RESET ROLE')
  end

  def truncate_runtime_tables
    connection = ActiveRecord::Base.connection
    tables = connection.tables - %w[ar_internal_metadata schema_migrations]
    quoted_tables = tables.map { |table| connection.quote_table_name(table) }.join(', ')
    connection.execute("TRUNCATE TABLE #{quoted_tables} RESTART IDENTITY CASCADE")
  end

  def restore_fixture_baseline
    truncate_runtime_tables

    PaperTrail.request(enabled: false) do
      fixture_names = Rails.root.glob('spec/fixtures/*.yml').map { |path| path.basename('.yml').to_s }
      SpecFixtureLoader.load(fixture_names)
      FixtureHouseholdSetup.apply!
    end
  end
end
