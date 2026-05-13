# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Schedules::Card::DoseStatusComponent, type: :component do
  let(:person) { create(:person) }
  let(:medication) { create(:medication, name: 'Ibuprofen', current_supply: 1000, supply_at_last_restock: 1000) }
  let(:schedule) do
    Schedule.create!(
      person: person,
      medication: medication,
      dose_amount: 400.0,
      dose_unit: 'mg',
      frequency: 'Twice daily',
      max_daily_doses: 4,
      notes: 'Take with food',
      start_date: 1.month.ago,
      end_date: 1.month.from_now
    )
  end
  let(:presenter) { Schedules::CardPresenter.new(schedule: schedule, current_user: nil, person: person) }

  it 'renders dates and notes', :aggregate_failures do
    rendered = render_inline(described_class.new(schedule: schedule, presenter: presenter))

    expect(rendered.text).to include('Take with food')
    expect(rendered.text).to include('Started')
    expect(rendered.text).to include('Ends')
    expect(rendered.text).not_to include("Today's Doses")
    expect(rendered.text).not_to match(/[\u{1F300}-\u{1FAFF}\u{2600}-\u{27BF}]/)
  end
end
