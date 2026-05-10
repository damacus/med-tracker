# frozen_string_literal: true

require "rails_helper"

RSpec.describe Components::Schedules::Card::DoseStatusComponent, type: :component do
  let(:person) { create(:person) }
  let(:medication) { create(:medication, name: "Ibuprofen", current_supply: 1000, supply_at_last_restock: 1000) }
  let(:schedule) do
    Schedule.create!(
      person: person,
      medication: medication,
      dose_amount: 400.0,
      dose_unit: "mg",
      frequency: "Twice daily",
      max_daily_doses: 4,
      notes: "Take with food",
      start_date: 1.month.ago,
      end_date: 1.month.from_now
    )
  end

  let(:presenter) {
    Schedules::CardPresenter.new(schedule: schedule, todays_takes: [take], current_user: nil, person: person)
  }
  let(:take) { create(:medication_take, :for_schedule, schedule: schedule, taken_at: Time.current, dose_amount: 400) }

  it "renders dates, notes, dose count, and today take history", :aggregate_failures do
    rendered = render_inline(described_class.new(schedule: schedule, presenter: presenter))

    expect(rendered.text).to(include("Take with food"))
    expect(rendered.text).to(include("Today's Doses"))
    expect(rendered.text).to(include("1/4"))
    expect(rendered.text).to(include("400 mg"))
    expect(rendered.text).not_to(match(/[\u{1F300}-\u{1FAFF}\u{2600}-\u{27BF}]/))
    expect(rendered.css("svg.lucide-calendar")).to(be_present)
    expect(rendered.css("svg.lucide-file-text")).to(be_present)
  end
end
