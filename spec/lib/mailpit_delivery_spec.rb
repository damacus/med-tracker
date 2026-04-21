# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MailpitDelivery do
  let(:settings) { { address: '127.0.0.1', port: 1025 } }
  let(:delivery) { described_class.new(settings) }
  let(:mail) { instance_double(Mail::Message) }

  around do |example|
    original_delivery_method = ActionMailer::Base.delivery_method
    original_deliveries = ActionMailer::Base.deliveries.dup
    ActionMailer::Base.deliveries.clear

    example.run
  ensure
    ActionMailer::Base.delivery_method = original_delivery_method
    ActionMailer::Base.deliveries.replace(original_deliveries)
  end

  it 'stores the mail in deliveries and forwards it to SMTP when mailpit delivery is active' do
    smtp_client = instance_double(Mail::SMTP, deliver!: true)
    allow(Mail::SMTP).to receive(:new).with(settings).and_return(smtp_client)
    ActionMailer::Base.delivery_method = :mailpit

    delivery.deliver!(mail)

    expect(ActionMailer::Base.deliveries).to include(mail)
    expect(smtp_client).to have_received(:deliver!).with(mail)
  end

  it 'does not duplicate deliveries when the test delivery method is active' do
    smtp_client = instance_double(Mail::SMTP, deliver!: true)
    allow(Mail::SMTP).to receive(:new).with(settings).and_return(smtp_client)
    ActionMailer::Base.delivery_method = :test

    delivery.deliver!(mail)

    expect(ActionMailer::Base.deliveries).to be_empty
    expect(smtp_client).to have_received(:deliver!).with(mail)
  end

  it 'logs a warning and suppresses SMTP connection failures' do
    allow(Mail::SMTP).to receive(:new).with(settings).and_raise(
      SocketError,
      'getaddrinfo: nodename nor servname provided'
    )
    allow(Rails.logger).to receive(:warn)
    ActionMailer::Base.delivery_method = :mailpit

    expect { delivery.deliver!(mail) }.not_to raise_error

    expect(Rails.logger).to have_received(:warn).with(/\[MailpitDelivery\] SMTP delivery to Mailpit failed:/)
    expect(ActionMailer::Base.deliveries).to include(mail)
  end
end
