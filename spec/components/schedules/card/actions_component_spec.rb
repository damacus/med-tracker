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
      dock: ['grid-cols-[minmax(0,1fr)_3rem]', 'gap-y-4', 'bg-surface-container-high', 'p-4'],
      log: %w[col-span-2],
      edit: %w[min-w-0],
      delete: %w[order-3],
      log_button: %w[h-14 rounded-full shadow-elevation-2],
      edit_button: %w[h-14 rounded-3xl bg-surface-container-lowest],
      delete_button: %w[bg-transparent text-error],
      actions: %i[log edit delete]
    )
  end

  def schedule_action_dock_signature(rendered)
    nodes = schedule_action_nodes(rendered)

    schedule_action_layout_signature(nodes).merge(
      schedule_button_signature(nodes),
      actions: schedule_action_names(nodes)
    )
  end

  def schedule_action_nodes(rendered)
    dock = rendered.at_css('[data-testid="schedule-action-dock"]')
    {
      dock: dock,
      log: dock.at_css('[data-testid="schedule-log-action"]'),
      edit: dock.at_css('[data-testid="schedule-edit-action"]'),
      delete: dock.at_css('[data-testid="schedule-delete-action"]'),
      log_button: dock.at_css("button[data-testid='log-past-dose-schedule-#{schedule.id}']"),
      edit_button: dock.at_css("[data-testid='edit-schedule-#{schedule.id}']"),
      delete_button: dock.at_css("button[data-testid='delete-schedule-#{schedule.id}']")
    }
  end

  def schedule_action_layout_signature(nodes)
    {
      dock: class_tokens(nodes[:dock]).intersection(
        ['grid-cols-[minmax(0,1fr)_3rem]', 'gap-y-4', 'bg-surface-container-high', 'p-4']
      ),
      log: class_tokens(nodes[:log]).intersection(%w[col-span-2]),
      edit: class_tokens(nodes[:edit]).intersection(%w[min-w-0]),
      delete: class_tokens(nodes[:delete]).intersection(%w[order-3])
    }
  end

  def schedule_button_signature(nodes)
    {
      log_button: class_tokens(nodes[:log_button]).intersection(%w[h-14 rounded-full shadow-elevation-2]),
      edit_button: class_tokens(nodes[:edit_button]).intersection(%w[h-14 rounded-3xl bg-surface-container-lowest]),
      delete_button: class_tokens(nodes[:delete_button]).intersection(%w[text-error bg-transparent])
    }
  end

  def schedule_action_names(nodes)
    [
      (:log if nodes[:log_button]),
      (:edit if nodes[:edit_button]),
      (:delete if nodes[:delete_button])
    ].compact
  end

  def class_tokens(node)
    node['class'].split
  end
end
