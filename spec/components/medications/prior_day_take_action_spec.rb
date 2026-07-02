# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::PriorDayTakeAction, type: :component do
  fixtures :accounts, :locations, :medications, :people, :users

  let(:person) { people(:jane) }
  let(:user) { users(:admin) }
  let(:medication) { medications(:ibuprofen) }
  let(:source) do
    Schedule.create!(
      person: person,
      medication: medication,
      start_date: Time.zone.today,
      end_date: Time.zone.today + 30.days,
      dose_amount: medication.dose_amount,
      dose_unit: medication.dose_unit
    )
  end

  it 'renders a datetime-local field defaulted to now with the same max' do
    travel_to(Time.zone.local(2026, 4, 28, 14, 45)) do
      build_alternate_medication

      rendered = render_component
      timestamp_field = rendered.at_css("input[type='datetime-local'][name='medication_take[taken_at]']")

      expect(rendered.text).to include('Record a dose from a previous day')
      expect(rendered.at_css("form[action='#{take_path}']")).not_to be_nil
      expect(timestamp_field).not_to be_nil
      expect(timestamp_field['value']).to eq('2026-04-28T14:45')
      expect(timestamp_field['max']).to eq('2026-04-28T14:45')
    end
  end

  it 'renders a menuitem trigger with the configured testid and a calendar icon' do
    build_alternate_medication
    rendered = render_component

    trigger = rendered.at_css("[role='menuitem'][data-testid='log-past-dose-schedule-#{source.id}']")
    expect(trigger).not_to be_nil
    expect(trigger.text).to include('Log a past dose')
    expect(trigger.at_css('svg.lucide-calendar')).not_to be_nil
  end

  it 'applies a custom dialog trigger wrapper class for full-width action buttons' do
    build_alternate_medication
    rendered = render_button_component

    button = rendered.at_css("button[data-testid='log-past-dose-schedule-#{source.id}']")
    trigger_wrapper = button.ancestors.find { |node| node['data-action'] == 'click->ruby-ui--dialog#open' }

    expect(button).not_to be_nil
    expect(trigger_wrapper['class'].split).to include('block', 'w-full')
  end

  def render_component
    html = view_context.render(
      described_class.new(
        source: source,
        context: { person: person, current_user: user },
        amount: source.dose_amount,
        testid: "log-past-dose-schedule-#{source.id}"
      )
    )
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  def render_button_component
    html = view_context.render(
      described_class.new(
        source: source,
        context: { person: person, current_user: user },
        amount: source.dose_amount,
        testid: "log-past-dose-schedule-#{source.id}",
        button: {
          variant: :outlined,
          size: :lg,
          trigger_class: 'block w-full',
          class: 'w-full'
        }
      )
    )
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  def take_path
    Rails.application.routes.url_helpers.take_medication_person_schedule_path(
      household_slug: 'test-household',
      person_id: person,
      id: source
    )
  end

  def build_alternate_medication
    Medication.create!(
      name: medication.name,
      location: locations(:school),
      category: medication.category,
      dose_amount: medication.dose_amount,
      dose_unit: medication.dose_unit,
      current_supply: 7,
      reorder_threshold: 1
    )
  end
end
