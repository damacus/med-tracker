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
    expect(rendered.text).to include(schedule.medication.current_supply.to_s)
  end
end
