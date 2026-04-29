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
  let(:presenter) { Schedules::CardPresenter.new(schedule: schedule, todays_takes: [], current_user: nil, person: person) }

  it 'renders the take action with schedule context' do
    rendered = render_inline(
      described_class.new(schedule: schedule, person: person, presenter: presenter, current_user: nil)
    )

    expect(rendered.css("[data-testid='take-schedule-#{schedule.id}']")).to be_present
  end

  context 'when the dose is invalid' do
    it 'renders a disabled take action' do
      schedule.dose_amount = 0
      rendered = render_inline(
        described_class.new(schedule: schedule, person: person, presenter: presenter, current_user: nil)
      )

      button = rendered.at_css("button[data-testid='take-schedule-#{schedule.id}-disabled'][disabled]")
      expect(button).to be_present
      expect(button.text).to include('Invalid Dose Configured')
    end
  end

  context 'when the schedule is on cooldown' do
    it 'renders a disabled waiting action' do
      allow(MedicationStockSourceResolver).to receive(:new).and_return(
        instance_double(MedicationStockSourceResolver, blocked_reason: :cooldown)
      )

      rendered = render_inline(
        described_class.new(schedule: schedule, person: person, presenter: presenter, current_user: nil)
      )

      button = rendered.at_css("button[data-testid='take-schedule-#{schedule.id}-disabled'][disabled]")
      expect(button).to be_present
      expect(button.text).to include('Waiting')
    end
  end
end
