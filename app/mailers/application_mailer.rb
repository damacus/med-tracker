# frozen_string_literal: true

class ApplicationMailer < ActionMailer::Base
  default from: lambda {
    Rails.application.credentials.dig(:mailer, :from) ||
      ENV.fetch('MAILER_FROM', 'MedTracker <noreply@medtracker.app>')
  }
  layout false

  after_action :inline_mail_styles

  private

  def render_mail_component(component)
    view_context.render(component)
  end

  def inline_mail_styles
    return unless defined?(Premailer::Rails::Hook)
    return if message.html_part.blank? && message.content_type.to_s.exclude?('text/html')

    Premailer::Rails::Hook.perform(message)
  end
end
