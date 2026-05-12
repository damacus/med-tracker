# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::ScheduleRow, type: :component do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules

  subject(:row) do
    described_class.new(
      person: person,
      schedule: schedule
    )
  end

  let(:person) { people(:john) }
  let(:schedule) { schedules(:active_schedule) }

  it 'renders the medication quantity' do
    rendered = render_inline(row)
    expect(rendered.text).to include(MedicationStockQuantityFormatter.format(schedule.medication.current_supply))
  end

  it 'renders the shared person avatar with initials instead of emoji' do
    rendered = render_inline(row)

    expect(rendered.text).not_to include('👤')
    expect(rendered.at_css('[data-testid="person-avatar"]')).to be_present
    expect(rendered.text).to include('JD')
  end
end
