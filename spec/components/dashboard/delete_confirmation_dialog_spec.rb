# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::DeleteConfirmationDialog, type: :component do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules

  let(:schedule) { schedules(:active_schedule) }

  describe 'rendering' do
    it 'renders a delete trigger button' do
      rendered = render_inline(described_class.new(schedule: schedule))

      expect(rendered.text).to include('Delete')
    end

    it 'renders the confirmation dialog content' do
      rendered = render_inline(described_class.new(schedule: schedule))

      expect(rendered.text).to include('Delete Schedule?')
    end

    it 'includes the medication name in the confirmation message' do
      rendered = render_inline(described_class.new(schedule: schedule))

      expect(rendered.text).to include(schedule.medication.name)
    end

    it 'includes the person name in the confirmation message' do
      rendered = render_inline(described_class.new(schedule: schedule))

      expect(rendered.text).to include(schedule.person.name)
    end
  end

  describe 'dialog actions' do
    it 'renders a cancel button' do
      rendered = render_inline(described_class.new(schedule: schedule))

      expect(rendered.text).to include('Cancel')
    end

    it 'renders a delete confirmation form with DELETE method' do
      rendered = render_inline(described_class.new(schedule: schedule))

      form = rendered.css('form').last
      expect(form).to be_present
    end

    it 'renders a destructive confirm button' do
      rendered = render_inline(described_class.new(schedule: schedule))

      confirm_button = rendered.css('[data-test-id^="confirm-delete"]')
      expect(confirm_button).to be_present
    end
  end

  describe 'trigger button' do
    it 'renders with a test id based on schedule id' do
      rendered = render_inline(described_class.new(schedule: schedule))

      trigger = rendered.css("[data-test-id='delete-schedule-#{schedule.id}']")
      expect(trigger).to be_present
    end

    it 'applies custom button_class when provided' do
      rendered = render_inline(described_class.new(
                                 schedule: schedule,
                                 button_class: 'custom-class'
                               ))

      trigger = rendered.css("[data-test-id='delete-schedule-#{schedule.id}']").first
      expect(trigger['class']).to include('custom-class')
    end
  end
end
