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

  it 'renders administrator actions in a responsive action dock' do
    admin = instance_double(User, administrator?: true)

    rendered = render_inline(
      described_class.new(schedule: schedule, person: person, presenter: presenter, current_user: admin)
    )

    expect(schedule_action_dock_signature(rendered)).to eq(
      shell: ['@container'],
      dock: ['grid-cols-[minmax(0,1fr)_3rem]', '@[22rem]:grid-cols-[minmax(5.25rem,0.7fr)_minmax(0,1.4fr)_3rem]'],
      log: %w[order-1 col-span-2 @[22rem]:order-2],
      edit: %w[order-2 @[22rem]:order-1],
      delete: %w[order-3],
      actions: %i[log edit delete]
    )
  end

  def schedule_action_dock_signature(rendered)
    shell = rendered.at_css('[data-testid="schedule-action-shell"]')
    dock = rendered.at_css('[data-testid="schedule-action-dock"]')
    log_action = dock.at_css('[data-testid="schedule-log-action"]')
    edit_action = dock.at_css('[data-testid="schedule-edit-action"]')
    delete_action = dock.at_css('[data-testid="schedule-delete-action"]')

    {
      shell: class_tokens(shell).intersection(['@container']),
      dock: class_tokens(dock).intersection(
        ['grid-cols-[minmax(0,1fr)_3rem]', '@[22rem]:grid-cols-[minmax(5.25rem,0.7fr)_minmax(0,1.4fr)_3rem]']
      ),
      log: class_tokens(log_action).intersection(%w[order-1 col-span-2 @[22rem]:order-2]),
      edit: class_tokens(edit_action).intersection(%w[order-2 @[22rem]:order-1]),
      delete: class_tokens(delete_action).intersection(%w[order-3]),
      actions: schedule_action_names(log_action, edit_action, delete_action)
    }
  end

  def schedule_action_names(log_action, edit_action, delete_action)
    [
      (:log if log_action.at_css("button[data-testid='log-past-dose-schedule-#{schedule.id}']")),
      (:edit if edit_action.at_css("[data-testid='edit-schedule-#{schedule.id}']")),
      (:delete if delete_action.at_css("button[data-testid='delete-schedule-#{schedule.id}']"))
    ].compact
  end

  def class_tokens(node)
    node['class'].split
  end
end
