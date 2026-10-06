# frozen_string_literal: true

require 'json'

namespace :hosted_restore do
  desc 'Run owner-role migrations and report a sanitized schema result'
  task migrate: :environment do
    role = ActiveRecord::Base.connection.select_value('SELECT current_user')
    raise HostedRestore::VerificationError, 'migration_owner_role_required' unless role == 'med_tracker_owner'

    Rake::Task['db:migrate'].tap(&:reenable).invoke
    schema_version = ActiveRecord::Base.connection.select_value('SELECT max(version) FROM schema_migrations').to_s
    puts JSON.generate(outcome: 'passed', database_role: role, schema_version:)
  rescue HostedRestore::VerificationError => e
    warn JSON.generate(outcome: 'failed', failure_code: e.message)
    exit(1)
  end

  desc 'Verify runtime RLS, tenant isolation, and restored attachment checksums'
  task verify_runtime: :environment do
    result = HostedRestore::RuntimeVerifier.new(
      household_ids: [ENV.fetch('HOUSEHOLD_A_ID'), ENV.fetch('HOUSEHOLD_B_ID')]
    ).call
    puts JSON.generate({ outcome: 'passed' }.merge(result))
  rescue HostedRestore::VerificationError, KeyError => e
    failure_code = e.is_a?(HostedRestore::VerificationError) ? e.message : 'required_input_missing'
    warn JSON.generate(outcome: 'failed', failure_code:)
    exit(1)
  end

  desc 'Verify database audit chains, Object Lock objects, and external WORM heads'
  task verify_audit: :environment do
    result = HostedRestore::AuditVerifier.new(
      household_ids: [ENV.fetch('HOUSEHOLD_A_ID'), ENV.fetch('HOUSEHOLD_B_ID')],
      expected_heads: JSON.parse(ENV.fetch('WORM_HEADS_JSON'))
    ).call
    puts JSON.generate({ outcome: 'passed' }.merge(result))
  rescue HostedRestore::VerificationError, JSON::ParserError, KeyError => e
    failure_code = if e.is_a?(HostedRestore::VerificationError)
                     e.message
                   elsif e.is_a?(JSON::ParserError)
                     'worm_heads_invalid'
                   else
                     'required_input_missing'
                   end
    warn JSON.generate(outcome: 'failed', failure_code:)
    exit(1)
  end
end
