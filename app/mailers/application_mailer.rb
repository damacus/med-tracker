# frozen_string_literal: true

class ApplicationMailer < ActionMailer::Base
  default from: lambda {
    Rails.application.credentials.dig(:mailer, :from) ||
      ENV.fetch('MAILER_FROM', 'MedTracker <noreply@medtracker.app>')
  }
  layout 'mailer'
end
