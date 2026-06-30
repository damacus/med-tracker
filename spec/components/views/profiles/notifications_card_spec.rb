# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Profiles::NotificationsCard, type: :component do
  let(:person) { create(:person) }

  it 'renders a hidden test notification button wired to the push notification controller' do
    rendered = render_inline(described_class.new(person: person))

    expect(rendered.at_css('#notifications-card')).to be_present

    test_button = rendered.at_css('button[data-push-notification-target="testButton"]')

    expect(test_button).to be_present
    expect(test_button.text).to include('Send Test Notification')
    expect(test_button.attribute('hidden')).to be_present
    expect(test_button['data-action']).to eq('push-notification#sendTest')
  end

  it 'renders notification category controls in the profile form' do
    rendered = render_inline(described_class.new(person: person))

    %w[dose_due_enabled missed_dose_enabled low_stock_enabled private_text_enabled].each do |category|
      expect(rendered.at_css("input[name='notification_preference[#{category}]'][type='checkbox']"))
        .to be_present
    end
  end
end
