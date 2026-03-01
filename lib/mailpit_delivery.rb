# frozen_string_literal: true

class MailpitDelivery
  def initialize(values)
    @settings = values
  end

  def deliver!(mail)
    ActionMailer::Base.deliveries << mail
    Mail::SMTP.new(@settings).deliver!(mail)
  rescue Errno::ECONNREFUSED, Net::OpenTimeout, SocketError => e
    Rails.logger.warn "[MailpitDelivery] SMTP delivery to Mailpit failed: #{e.message}"
  end
end
