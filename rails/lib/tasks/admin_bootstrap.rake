# frozen_string_literal: true

namespace :med_tracker do
  desc 'Bootstrap the first administrator account'
  task bootstrap_admin: :environment do
    required_keys = %w[ADMIN_EMAIL ADMIN_PASSWORD ADMIN_NAME ADMIN_DOB].freeze
    missing_keys = required_keys.select { |key| ENV[key].blank? }

    if missing_keys.any?
      puts "Missing required environment variables: #{missing_keys.join(', ')}"
      next
    end

    result = Admin::BootstrapService.call(
      email: ENV.fetch('ADMIN_EMAIL'),
      password: ENV.fetch('ADMIN_PASSWORD'),
      name: ENV.fetch('ADMIN_NAME'),
      date_of_birth: ENV.fetch('ADMIN_DOB')
    )

    if result.success?
      puts "Admin bootstrap successful: created #{result.user.email_address}"
    else
      puts "Admin bootstrap failed: #{result.error}"
    end
  end
end
