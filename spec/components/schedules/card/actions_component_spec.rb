# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Schedules::Card::ActionsComponent, type: :component do
  before do
    @person = create(:person)
    @medication = create(:medication, name: 'Ibuprofen', current_supply: 1000, supply_at_last_restock: 1000)
    @household = person.household
    @account = Account.create!(email: 'schedule-actions-owner@example.test', status: :verified)
    person.update!(account: account)
    @membership = household.household_memberships.create!(account: account, person: person, role: :owner,
                                                          status: :active)
    @schedule = Schedule.create!(
      household: household,
      person: person,
      medication: medication,
      dose_amount: 400.0,
      dose_unit: 'mg',
      frequency: 'Twice daily',
      start_date: 1.month.ago,
      end_date: 1.month.from_now
    )

    household.person_access_grants.create!(
      household_membership: membership,
      person: person,
      access_level: :manage,
      relationship_type: :self,
      granted_by_membership: membership
    )
  end

  after do
    Current.reset
  end

  attr_reader :account, :household, :medication, :membership, :person, :schedule

  def presenter
    Schedules::CardPresenter.new(schedule: schedule, current_user: nil, person: person)
  end

  it 'renders the log past dose action' do
    rendered = render_inline(
      described_class.new(schedule: schedule, person: person, presenter: presenter, current_user: nil)
    )

    expect(rendered.css("[data-testid='log-past-dose-schedule-#{schedule.id}']")).to be_present
  end

  it 'renders secondary actions in an actions menu', :aggregate_failures do
    rendered = render_as_owner

    trigger = rendered.at_css("button[data-testid='schedule-actions-#{schedule.id}']")
    trigger_classes = trigger['class'].split
    edit_link = rendered.at_css("[data-testid='edit-schedule-#{schedule.id}']")
    pause_button = rendered.at_css("[data-testid='pause-schedule-#{schedule.id}']")

    expect(trigger).to be_present
    expect(trigger_classes).to include('h-11')
    expect(trigger_classes).to include('w-11')
    expect(trigger.at_css('.sr-only').text).to eq('Actions')
    expect(rendered.css("[data-testid='edit-schedule-#{schedule.id}']")).to be_present
    expect(rendered.css("[data-testid='delete-schedule-#{schedule.id}']")).to be_present
    expect(edit_link['role']).to eq('menuitem')
    expect(edit_link.text).to include('Edit')
    expect(pause_button['role']).to eq('menuitem')
    expect(pause_button.text).to include('Pause schedule')
  end

  it 'keeps action controls on one line with a labelled actions trigger', :aggregate_failures do
    rendered = render_as_owner
    action_row = rendered.at_css('[data-testid="schedule-card-actions"]')
    trigger = rendered.at_css("button[data-testid='schedule-actions-#{schedule.id}']")

    expect(action_row['class'].split).not_to include('flex-wrap')
    expect(action_row['class'].split).to include('min-w-0')
    expect(trigger['class'].split).to include('rounded-shape-full')
    expect(trigger['class'].split).to include('shrink-0')
  end

  it 'uses design-system sizing for compact action controls', :aggregate_failures do
    rendered = render_as_owner
    past_dose_button_classes = rendered.at_css("button[data-testid='log-past-dose-schedule-#{schedule.id}']")[
      'class'
    ].split
    dropdown = rendered.at_css('[data-ruby-ui--dropdown-menu-options-value]')

    expect(past_dose_button_classes).to include('min-h-[44px]')
    expect(past_dose_button_classes).not_to include('h-14')
    expect(past_dose_button_classes).not_to include('min-h-[56px]')
    expect(past_dose_button_classes).not_to include('py-6')
    expect(dropdown['data-ruby-ui--dropdown-menu-options-value']).to include('"strategy":"fixed"')
  end

  def render_as_owner
    Current.account = account
    Current.household = household
    Current.membership = membership

    render_inline(described_class.new(schedule: schedule, person: person, presenter: presenter, current_user: nil))
  end
end
