# frozen_string_literal: true

require "rails_helper"

RSpec.describe Components::Medications::PriorDayTakeAction, type: :component do
  fixtures(:accounts, :locations, :medications, :people, :users)

  let(:person) { people(:jane) }
  let(:user) { users(:admin) }
  let(:medication) { medications(:ibuprofen) }
  let(:source) do
    Schedule.create!(
      person: person,
      medication: medication,
      start_date: Time.zone.today,
      end_date: Time.zone.today + 30.days,
      dose_amount: medication.dosage_amount,
      dose_unit: medication.dosage_unit
    )
  end

  it "renders a datetime-local field defaulted to now with the same max" do
    travel_to(Time.zone.local(2026, 4, 28, 14, 45)) do
      build_alternate_medication

      rendered = render_component
      timestamp_field = rendered.at_css("input[type='datetime-local'][name='medication_take[taken_at]']")

      expect(rendered.text).to(include("Record a dose from a previous day"))
      expect(rendered.at_css("form[action='#{take_path}']")).not_to(be_nil)
      expect(timestamp_field).not_to(be_nil)
      expect(timestamp_field["value"]).to(eq("2026-04-28T14:45"))
      expect(timestamp_field["max"]).to(eq("2026-04-28T14:45"))
    end
  end

  it "renders a menuitem trigger with the configured testid and a calendar icon" do
    build_alternate_medication
    rendered = render_component

    trigger = rendered.at_css("[role='menuitem'][data-testid='log-past-dose-schedule-#{source.id}']")
    expect(trigger).not_to(be_nil)
    expect(trigger.text).to(include("Log a past dose"))
    expect(trigger.at_css("svg.lucide-calendar")).not_to(be_nil)
  end

  def render_component
    html = view_context.render(
      described_class.new(
        source: source,
        context: {person: person, current_user: user},
        amount: source.dose_amount,
        testid: "log-past-dose-schedule-#{source.id}"
      )
    )
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  def take_path
    Rails.application.routes.url_helpers.take_medication_person_schedule_path(person, source)
  end

  def build_alternate_medication
    Medication.create!(
      name: medication.name,
      location: locations(:school),
      category: medication.category,
      dosage_amount: medication.dosage_amount,
      dosage_unit: medication.dosage_unit,
      current_supply: 7,
      reorder_threshold: 1
    )
  end
end
