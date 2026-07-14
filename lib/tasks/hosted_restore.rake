# frozen_string_literal: true

require 'json'

namespace :hosted_restore do
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
    failure_code = e.is_a?(HostedRestore::VerificationError) ? e.message : 'required_input_missing'
    warn JSON.generate(outcome: 'failed', failure_code:)
    exit(1)
  end
end
