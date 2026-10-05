# frozen_string_literal: true

class MailpitDelivery
  def initialize(values)
    @settings = values
  end

  def deliver!(mail)
    ActionMailer::Base.deliveries << mail unless ActionMailer::Base.delivery_method == :test
    Mail::SMTP.new(@settings).deliver!(mail)
  rescue Errno::ECONNREFUSED, Net::OpenTimeout, SocketError => e
    Observability::DiagnosticEvent.emit(
      component: :mailpit,
      reason: :operation_failed,
      severity: :warn,
      error: e
    )
  end
end
