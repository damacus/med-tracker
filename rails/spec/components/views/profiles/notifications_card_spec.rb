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

  it 'renders browser notification status as an accessible live region' do
    rendered = render_inline(described_class.new(person: person))
    status = rendered.at_css('[data-push-notification-target="status"]')

    expect(status).to be_present
    expect(status['role']).to eq('status')
    expect(status['aria-live']).to eq('polite')
    expect(status['aria-atomic']).to eq('true')
  end

  it 'renders notification category controls as compact on and off toggle groups' do
    rendered = render_inline(described_class.new(person: person))

    %w[dose_due_enabled missed_dose_enabled low_stock_enabled private_text_enabled].each do |category|
      input = rendered.at_css("input[name='notification_preference[#{category}]'][type='hidden']")
      group = input.ancestors('[role="radiogroup"]').first

      expect(group.css('[role="radio"]').map { |radio| radio.text.squish }).to eq(%w[On Off])
    end
  end

  it 'shows managed dependents as automatic and managed adults as optional' do
    child = build_stubbed(:person, :minor, name: 'Alex Child')
    adult = build_stubbed(:person, name: 'Sam Adult')
    child_grant = instance_double(PersonAccessGrant, person: child, missed_dose_notifications_enabled?: false)
    adult_grant = instance_double(PersonAccessGrant, person: adult, missed_dose_notifications_enabled?: true)

    rendered = render_inline(described_class.new(person: person, managed_grants: [child_grant, adult_grant]))

    expect(rendered.text).to include('Alex Child', 'Included automatically', 'Sam Adult')
    expect(rendered.at_css("input[name='notification_preference[managed_person_ids][]'][value='#{child.id}']"))
      .not_to be_present

    adult_toggle = rendered.at_css(
      "input[name='notification_preference[managed_person_ids][]'][value='#{adult.id}'][type='hidden']"
    )
    expect(adult_toggle).to be_present
    expect(adult_toggle.ancestors('[role="radiogroup"]').first.at_css('[aria-checked="true"]').text).to eq('On')
  end

  it 'uses shared action styles for notification buttons' do
    rendered = render_inline(described_class.new(person: person))
    action_classes = rendered.css('button:not([role="radio"]):not([aria-controls])').map do |button|
      button[:class].to_s.split
    end

    expect(action_classes).to all(include_touch_target_class)
    expect(action_classes).to all(include('rounded-shape-full'))
    expect(action_classes.flatten).not_to include('text-white')
    expect(action_classes.flatten).not_to include('rounded-lg')
    expect(action_classes.flatten).not_to include('rounded-xl')
  end

  def include_touch_target_class
    satisfy { |classes| classes.include?('min-h-11') || classes.include?('min-h-[44px]') }
  end
end
