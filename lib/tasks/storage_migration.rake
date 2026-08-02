# frozen_string_literal: true

require 'json'

namespace :storage do
  desc 'Run a bounded portable-storage migration operation'
  task migration: :environment do
    status = StorageMigration::Command.new.call
    exit(status) unless status.zero?
  end

  desc 'Convert or report live path-backed NHS dm+d archives'
  task convert_nhs_dmd_archives: :environment do
    result = NhsDmd::ArchiveMigration.new(
      service_name: ENV.fetch('NHS_DMD_ARCHIVE_SERVICE'),
      apply: ENV['NHS_DMD_ARCHIVE_APPLY'] == 'true'
    ).call
    puts JSON.generate({ outcome: result.failed_count.zero? ? 'passed' : 'failed' }.merge(result.to_h))
    exit(1) unless result.failed_count.zero?
  rescue SecurityError, ArgumentError, KeyError => e
    failure_code = e.is_a?(SecurityError) ? 'owner_role_required' : 'invalid_input'
    warn JSON.generate(outcome: 'failed', failure_code:)
    exit(2)
  end
end
