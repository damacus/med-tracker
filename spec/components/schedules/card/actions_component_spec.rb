# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Schedules::Card::ActionsComponent, type: :component do
  let(:person) { create(:person) }
  let(:medication) { create(:medication, name: 'Ibuprofen', current_supply: 1000, supply_at_last_restock: 1000) }
  let(:schedule) do
    Schedule.create!(
      person: person,
      medication: medication,
      dose_amount: 400.0,
      dose_unit: 'mg',
      frequency: 'Twice daily',
      start_date: 1.month.ago,
      end_date: 1.month.from_now
    )
  end
  let(:presenter) { Schedules::CardPresenter.new(schedule: schedule, current_user: nil, person: person) }

  it 'renders the log past dose action' do
    rendered = render_inline(
      described_class.new(schedule: schedule, person: person, presenter: presenter, current_user: nil)
    )

    expect(rendered.css("[data-testid='log-past-dose-schedule-#{schedule.id}']")).to be_present
  end

  it 'renders the edit and delete links' do
    admin = instance_double(User, administrator?: true)

    rendered = render_inline(
      described_class.new(schedule: schedule, person: person, presenter: presenter, current_user: admin)
    )

    expect(rendered.css("[data-testid='edit-schedule-#{schedule.id}']")).to be_present
    expect(rendered.css("[data-testid='delete-schedule-#{schedule.id}']")).to be_present
  end
end
