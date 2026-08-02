# frozen_string_literal: true

require 'json'

namespace :med_tracker do
  namespace :storage do
    desc 'Verify an attachment restored from the database and persistent storage backup'
    task verify_restore: :environment do
      role = ActiveRecord::Base.connection.select_value('SELECT current_user')
      raise SecurityError unless role == 'med_tracker_owner'

      access_verifier = lambda do |**|
        {
          authorized_retrieval: ENV['AUTHORIZED_RETRIEVAL_VERIFIED'] == 'true',
          cross_household_denied: ENV['CROSS_HOUSEHOLD_DENIAL_VERIFIED'] == 'true'
        }
      end
      result = Storage::RestoreVerifier.call(
        attachment_id: ENV['ATTACHMENT_ID'].presence,
        access_verifier:
      )
      migration_run = StorageMigrationRun.find_by(run_id: ENV['STORAGE_MIGRATION_RUN_ID'].presence)
      recovery = Storage::RecoverySet.new(
        backends: result.required_backends,
        migration_run:
      ).record(
        database_reference: ENV.fetch('DATABASE_RECOVERY_REFERENCE', nil),
        disk_reference: ENV.fetch('DISK_RECOVERY_REFERENCE', nil),
        s3_reference: ENV.fetch('S3_RECOVERY_REFERENCE', nil)
      )
      puts JSON.generate(
        outcome: 'passed',
        attachment_id: result.attachment_id,
        blob_id: result.blob_id,
        byte_size: result.byte_size,
        required_backends: result.required_backends,
        authorized_retrieval: result.authorized_retrieval,
        cross_household_denied: result.cross_household_denied,
        database_reference: recovery.database_reference,
        storage_references: recovery.storage_references,
        migration_phase: recovery.migration_phase
      )
    rescue SecurityError
      warn JSON.generate(outcome: 'failed', failure_code: 'owner_role_required')
      exit(1)
    rescue Storage::RestoreVerifier::VerificationError
      warn JSON.generate(outcome: 'failed', failure_code: 'restore_verification_failed')
      exit(1)
    rescue Storage::RecoverySet::Error, KeyError
      warn JSON.generate(outcome: 'failed', failure_code: 'recovery_reference_invalid')
      exit(1)
    end
  end
end
