# frozen_string_literal: true

namespace :households do
  task migrate_local: :environment do
    dry_run = ENV['DRY_RUN'] == '1'
    apply = ENV['APPLY'] == '1'

    unless dry_run ^ apply
      puts 'Set exactly one of DRY_RUN=1 or APPLY=1'
      exit 1
    end

    result = Households::LocalMigrator.new(
      owner_email: ENV.fetch('OWNER_EMAIL', nil),
      household_name: ENV.fetch('HOUSEHOLD_NAME', nil),
      apply: apply
    ).call

    puts result.summary_lines.join("\n")
  rescue Households::LocalMigrator::Error => e
    puts e.message
    exit 1
  end
end
