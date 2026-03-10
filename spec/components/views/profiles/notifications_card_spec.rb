# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Profiles::NotificationsCard, type: :component do
  fixtures :people

  it 'renders a hidden test notification button wired to the push notification controller' do
    rendered = render_inline(described_class.new(person: people(:one)))

    test_button = rendered.at_css('button[data-push-notification-target="testButton"]')

    expect(test_button).to be_present
    expect(test_button.text).to include('Send Test Notification')
    expect(test_button.attribute('hidden')).to be_present
    expect(test_button['data-action']).to eq('push-notification#sendTest')
  end
end
