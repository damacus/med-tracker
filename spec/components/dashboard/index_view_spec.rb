# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::IndexView, type: :component do
  fixtures :accounts, :account_otp_keys, :people, :users, :locations, :medications, :dosages, :schedules,
           :person_medications, :medication_takes

  subject(:dashboard_view) do
    described_class.new(presenter: presenter)
  end

  let(:active_schedules_icon_path) do
    [
      'M200-640h560v-80H200v80Zm0 0v-80 80Zm0 560q-33 0-56.5-23.5T120-160v-560q0-33 ',
      '23.5-56.5T200-800h40v-80h80v80h320v-80h80v80h40q33 0 56.5 23.5T840-720v227q-19-9-39-15t-41-9v-43H200v400h252q7 ',
      '22 16.5 42T491-80H200Zm378.5-18.5Q520-157 520-240t58.5-141.5Q637-440 ',
      '720-440t141.5 58.5Q920-323 920-240T861.5-98.5Q803-40 ',
      '720-40T578.5-98.5ZM787-145l28-28-75-75v-112h-40v128l87 87Z'
    ].join
  end

  let(:admin_user) { users(:admin) }
  let(:presenter) { DashboardPresenter.new(current_user: admin_user) }

  describe 'greetings' do
    it 'renders Good morning in the morning' do
      travel_to Time.zone.parse('2026-02-25 09:00:00') do
        rendered = render_inline(dashboard_view)
        expect(rendered.text).to include('Good morning')
      end
    end

    it 'renders Good afternoon in the afternoon' do
      travel_to Time.zone.parse('2026-02-25 15:00:00') do
        rendered = render_inline(dashboard_view)
        expect(rendered.text).to include('Good afternoon')
      end
    end

    it 'renders Good evening in the evening' do
      travel_to Time.zone.parse('2026-02-25 20:00:00') do
        rendered = render_inline(dashboard_view)
        expect(rendered.text).to include('Good evening')
      end
    end
  end

  it 'renders the schedule heading' do
    rendered = render_inline(dashboard_view)
    expect(rendered.text).to include("Today's Schedule")
  end

  describe 'stats display' do
    it 'renders people count' do
      rendered = render_inline(dashboard_view)
      expect(rendered.text).to include(presenter.people.count.to_s)
    end

    it 'renders active schedules count' do
      rendered = render_inline(dashboard_view)
      expect(rendered.text).to include(presenter.active_schedules.count.to_s)
    end

    it 'renders all-family people count including self' do
      parent_presenter = DashboardPresenter.new(current_user: users(:parent), selected_person_id: 'all')
      parent_view = described_class.new(presenter: parent_presenter)
      expected_count = users(:parent).person.patients.where(person_type: :minor).count + 1

      rendered = render_inline(parent_view)

      expect(rendered.text).to include(expected_count.to_s)
    end

    it 'links People stat card to people page' do
      rendered = render_inline(dashboard_view)

      expect(rendered.css("a[href='/people']")).to be_present
    end

    it 'links Active Schedules stat card to schedules page' do
      rendered = render_inline(dashboard_view)

      expect(rendered.css("a[href='/schedules']")).to be_present
    end

    it 'renders the dashboard summary row with operational metric cards' do
      rendered = render_inline(dashboard_view)

      labels = rendered.css('p').map { |label| label.text.strip }

      expect(labels).to include('People', 'Active Schedules', 'Next Dose')
      expect(rendered.css("a[href='/people']")).to be_present
      expect(rendered.css("a[href='/schedules']")).to be_present
    end

    it 'renders the active schedules icon on the active schedules stat card' do
      rendered = render_inline(dashboard_view)
      card = rendered.at_css("a[href='/schedules']")

      expect(card.at_css("path[d='#{active_schedules_icon_path}']")).to be_present
    end
  end

  describe 'person selector' do
    it 'renders above the dashboard stats section with individual people and all family' do
      presenter = DashboardPresenter.new(current_user: users(:parent))

      rendered = render_inline(described_class.new(presenter: presenter))
      selector = rendered.at_css('[data-testid="dashboard-person-selector"]')
      overflow = rendered.at_css('[data-testid="dashboard-person-overflow"]')

      expect(selector).to be_present
      expect(selector.text).to include('Parent Person')
      expect(overflow.text).to include('All Family')
      expect(rendered.to_html.index('dashboard-person-selector')).to be < rendered.to_html.index('Active Schedules')
    end

    it 'marks the selected person with aria-current and initials fallback' do
      presenter = DashboardPresenter.new(current_user: users(:parent))

      rendered = render_inline(described_class.new(presenter: presenter))
      selected = rendered.at_css('[data-testid="dashboard-person-option"][aria-current="true"]')

      expect(selected.text).to include('Parent Person')
      expect(selected.text).to include('PP')
      expect(selected.at_css('[data-testid="person-avatar"]')).to be_present
    end

    it 'wraps selector controls instead of requiring horizontal scrolling' do
      presenter = DashboardPresenter.new(current_user: users(:parent))

      rendered = render_inline(described_class.new(presenter: presenter))
      selector = rendered.at_css('[data-testid="dashboard-person-selector"]')

      expect(selector['class']).not_to include('overflow-x-auto')
      expect(selector['class']).to include('max-w-full')
    end

    it 'renders the first five people as pills and puts remaining options in a dropdown' do
      presenter = DashboardPresenter.new(current_user: admin_user)

      rendered = render_inline(described_class.new(presenter: presenter))
      direct_options = rendered.css('[data-testid="dashboard-person-option"]')
      overflow = rendered.at_css('[data-testid="dashboard-person-overflow"]')

      expect(direct_options.count).to eq(5)
      expect(overflow).to be_present
      expect(overflow['data-controller']).to include('ruby-ui--dropdown-menu')
      expect(overflow.at_css('select')).not_to be_present
      expect(overflow.text).to include('All Family')
    end

    it 'renders only the current selection plus a dropdown for other people on mobile' do
      presenter = DashboardPresenter.new(current_user: admin_user)

      rendered = render_inline(described_class.new(presenter: presenter))
      current = rendered.at_css('[data-testid="dashboard-person-mobile-current"]')
      overflow = rendered.at_css('[data-testid="dashboard-person-mobile-overflow"]')
      selected_label = presenter.dashboard_person_options.find { |option| option.fetch(:selected) }.fetch(:label)
      other_labels = presenter.dashboard_person_options.reject { |option| option.fetch(:selected) }.map do |option|
        option.fetch(:label)
      end

      expect(current.text).to include(selected_label)
      expect(overflow['data-controller']).to include('ruby-ui--dropdown-menu')
      expect(overflow.at_css('select')).not_to be_present
      other_labels.each do |label|
        expect(overflow.text).to include(label)
      end
    end
  end

  describe 'quick actions' do
    it 'renders Add Medication link' do
      rendered = render_inline(dashboard_view)
      expect(rendered.text).to include('Add Medication')
    end

    it 'hides Add Person for admin users' do
      rendered = render_inline(dashboard_view)
      expect(rendered.text).not_to include('Add Person')
    end
  end

  describe 'high-fidelity sections' do
    it 'renders Smart Insights' do
      rendered = render_inline(dashboard_view)
      expect(rendered.text).to include('Smart Insights')
    end

    it 'does not render a fake detected pattern without an evidence-backed insight' do
      insight_result = SmartInsights::Result.new(
        primary_insight: nil,
        insights: [],
        learning_state?: true,
        evidence_summary: '1 day tracked'
      )
      presenter = person_task_presenter(insight_result: insight_result)

      rendered = render_inline(described_class.new(presenter: presenter))

      expect(rendered.text).not_to include('Pattern detected')
      expect(rendered.text).to include('Learning your routine')
    end

    it 'renders the no-action state when enough evidence has no detected patterns' do
      insight_result = SmartInsights::Result.new(
        primary_insight: nil,
        insights: [],
        learning_state?: false,
        evidence_summary: '7 days tracked'
      )
      presenter = person_task_presenter(insight_result: insight_result)

      rendered = render_inline(described_class.new(presenter: presenter))

      expect(rendered.text).to include('No pattern needs attention')
      expect(rendered.text).not_to include('Learning your routine')
      expect(rendered.text).not_to include('Strong streak')
    end

    it 'links authorized users to the reports insights section' do
      presenter = person_task_presenter(insight_result: detected_insight_result, can_view_reports: true)

      rendered = render_inline(described_class.new(presenter: presenter))

      expect(rendered.css("a[href='/reports#insights']")).to be_present
    end

    it 'renders the reports insights link with visible link affordance', :aggregate_failures do
      presenter = person_task_presenter(insight_result: detected_insight_result, can_view_reports: true)

      rendered = render_inline(described_class.new(presenter: presenter))
      link = rendered.at_css("a[href='/reports#insights']")

      expect(link.text).to include('View Full Report')
      expect(link['class']).to include('bg-surface-container-low')
      expect(link['class']).to include('text-on-surface')
      expect(link['class']).to include('px-5')
      expect(link['class']).to include('min-h-[44px]')
      expect(link['class']).to include('max-w-full')
      expect(link['class']).not_to include('p-0')
    end

    it 'omits the reports link when the user cannot view reports' do
      presenter = person_task_presenter(insight_result: detected_insight_result, can_view_reports: false)

      rendered = render_inline(described_class.new(presenter: presenter))

      expect(rendered.css("a[href='/reports#insights']")).not_to be_present
    end

    it 'renders Stock Inventory' do
      rendered = render_inline(dashboard_view)
      expect(rendered.text).to include('Stock Inventory')
    end

    it 'does not render a duplicate Next Dose card in the right rail' do
      rendered = render_inline(dashboard_view)

      expect(rendered.at_css('[data-testid="dashboard-right-rail-next-dose"]')).not_to be_present
    end

    it 'does not render a duplicate Medication Schedule section in the right rail' do
      rendered = render_inline(dashboard_view)

      expect(rendered.css('h2').map(&:text)).not_to include('Medication Schedule')
    end

    it 'renders the mobile content flow with inventory before insights' do
      rendered = render_inline(dashboard_view)
      html = rendered.to_html

      expect(html.index('Stock Inventory')).to be < html.index('Smart Insights')
    end
  end

  describe 'person task cards' do
    it 'renders a routine task card with a collapsed as-needed disclosure' do
      rendered = render_inline(described_class.new(presenter: person_task_presenter))
      details = rendered.at_css('details[data-testid="dashboard-as-needed-person"]')

      expect(rendered.text).to include('Vitamin D')
      expect(details).to be_present
      expect(details['open']).to be_nil
    end

    it 'renders as-needed availability inside the disclosure' do
      rendered = render_inline(described_class.new(presenter: person_task_presenter))
      details = rendered.at_css('details[data-testid="dashboard-as-needed-person"]')

      expect(details.text).to include('As needed')
      expect(details.text).to include('Paracetamol')
      expect(details.text).to include('Available now')
    end

    it 'counts blocked routine rows as remaining tasks' do
      rendered = render_inline(described_class.new(presenter: person_task_presenter(routine_status: :out_of_stock)))

      expect(rendered.text).to include('1 routine task left today')
      expect(rendered.text).not_to include('Routine tasks done today')
    end

    it 'does not duplicate the default upcoming state as a badge' do
      rendered = render_inline(described_class.new(presenter: person_task_presenter))
      routine_task = rendered.at_css('[data-testid="dashboard-routine-task"]')

      expect(routine_task.text).not_to include('Upcoming')
    end

    it 'renders dose progress pips for routine tasks with daily limits' do
      rendered = render_inline(described_class.new(presenter: person_task_presenter))
      routine_task = rendered.at_css('[data-testid="dashboard-routine-task"]')

      expect(routine_task.text).to include('0/1 doses today')
      expect(routine_task.css('[data-testid="dashboard-dose-pip"]').count).to eq(1)
    end

    it 'renders dose progress pips for as-needed items with daily limits' do
      rendered = render_inline(described_class.new(presenter: person_task_presenter))
      as_needed_task = rendered.at_css('[data-testid="dashboard-as-needed-task"]')

      expect(as_needed_task.text).to include('1/4 doses today')
      expect(as_needed_task.css('[data-testid="dashboard-dose-pip"]').count).to eq(4)
    end
  end

  describe 'today dose history' do
    it 'renders previous doses grouped by person before Smart Insights' do
      presenter = person_task_presenter

      rendered = render_inline(described_class.new(presenter: presenter))
      history = rendered.at_css('[data-testid="dashboard-today-dose-history"]')
      html = rendered.to_html

      expect(history).to be_present
      expect(history.text).to include('Previous Doses Today')
      expect(history.text).to include('John Doe')
      expect(history.text).to include('Paracetamol')
      expect(history.text).to include('09:15')
      expect(html.index('Previous Doses Today')).to be < html.index('Smart Insights')
    end
  end

  describe 'dashboard density' do
    it 'renders top metric cards with the compact layout' do
      rendered = render_inline(dashboard_view)
      html = rendered.to_html

      expect(html.scan('min-h-[7rem]').size).to eq(3)
      expect(html).to include('sm:grid-cols-3')
      expect(html).to include('min-h-[7rem]')
      expect(html).not_to include('min-h-[9.5rem]')
    end
  end

  context 'when there are no active schedules or person medications' do
    before do
      MedicationTake.delete_all
      PersonMedication.delete_all
      Schedule.update_all(end_date: 1.year.ago) # rubocop:disable Rails/SkipsModelValidations
    end

    it 'renders the empty state message' do
      rendered = render_inline(dashboard_view)
      expect(rendered.text).to include('No medications scheduled for today')
    end
  end

  def person_task_presenter(routine_status: :upcoming, insight_result: nil, can_view_reports: true)
    person = people(:john)
    instance_double(
      DashboardPresenter,
      people: [person],
      active_schedules: [],
      current_user: admin_user,
      next_dose_time: nil,
      smart_insights: insight_result || learning_insight_result,
      can_view_reports?: can_view_reports,
      dashboard_person_options: [],
      routine_tasks_due?: true,
      routine_tasks_by_person: { person => [routine_dashboard_row(person, status: routine_status)] },
      as_needed_by_person: { person => [as_needed_dashboard_row(person)] }
    )
  end

  def learning_insight_result
    SmartInsights::Result.new(
      primary_insight: nil,
      insights: [],
      learning_state?: true,
      evidence_summary: '1 day tracked'
    )
  end

  def detected_insight_result
    insight = detected_insight

    SmartInsights::Result.new(
      primary_insight: insight,
      insights: [insight],
      learning_state?: false,
      evidence_summary: '7 days tracked'
    )
  end

  def detected_insight
    SmartInsights::Insight.new(
      key: :adherence_streak,
      family: :adherence,
      severity: :positive,
      title: 'Strong streak',
      summary: '7 days logged',
      detail: 'Every expected dose was logged in this report window.',
      metric_label: 'Current streak',
      metric_value: '7 days',
      cta_path: '/reports#insights'
    )
  end

  def routine_dashboard_row(person, status: :upcoming)
    {
      person: person,
      source: person_medications(:john_vitamin_d),
      scheduled_at: nil,
      taken_at: nil,
      status: status,
      daily_dose_count: 0,
      daily_dose_limit: 1,
      today_takes: []
    }
  end

  def as_needed_dashboard_row(person)
    today_take = instance_double(
      MedicationTake,
      taken_at: Time.zone.parse('2026-05-05 09:15:00'),
      medication: medications(:paracetamol),
      dose_amount: 1000,
      dose_unit: 'mg'
    )

    {
      person: person,
      source: schedules(:john_paracetamol),
      scheduled_at: Time.current,
      taken_at: nil,
      status: :available,
      daily_dose_count: 1,
      daily_dose_limit: 4,
      today_takes: [today_take]
    }
  end
end
