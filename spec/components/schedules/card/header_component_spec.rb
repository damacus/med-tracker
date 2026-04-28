# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Schedules::Card::HeaderComponent, type: :component do
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
  let(:presenter) { Schedules::CardPresenter.new(schedule: schedule, todays_takes: [], current_user: nil, person: person) }

  it 'renders schedule metadata and stock status' do
    rendered = render_inline(described_class.new(schedule: schedule, presenter: presenter))

    expect(rendered.text).to include('Ibuprofen')
    expect(rendered.text).to include('400mg • Twice daily')
    expect(rendered.text).to include('Ready Now')
  end
end
