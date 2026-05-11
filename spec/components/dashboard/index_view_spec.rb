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

  let(:compliance_icon_path) do
    [
      'M480-80q-139-35-229.5-159.5T160-516v-244l320-120 320 120v200h-80v-145l-240-90-240 ',
      '90v189q0 121 68 220t172 132q26-8 49.5-20.5T576-214l56 56q-33 27-71.5 47T480-80Zm331.5-11.5Q800-103 ',
      '800-120t11.5-28.5Q823-160 840-160t28.5 11.5Q880-137 880-120t-11.5 28.5Q857-80 ',
      '840-80t-28.5-11.5ZM800-240v-240h80v240h-80ZM480-480Zm56.5 ',
      '56.5Q560-447 560-480t-23.5-56.5Q513-560 480-560t-56.5 23.5Q400-513 400-480t23.5 ',
      '56.5Q447-400 480-400t56.5-23.5ZM480-320q-66 ',
      '0-113-47t-47-113q0-66 47-113t113-47q66 0 113 47t47 113q0 22-5.5 42.5T618-398l119 ',
      '118-57 57-120-119q-18 11-38.5 16.5T480-320Z'
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

    it 'renders parent-scoped people count including self' do
      parent_presenter = DashboardPresenter.new(current_user: users(:parent))
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

    it 'links Compliance stat card to reports page' do
      rendered = render_inline(dashboard_view)

      expect(rendered.css("a[href='/reports']")).to be_present
    end

    it 'renders the active schedules icon on the active schedules stat card' do
      rendered = render_inline(dashboard_view)
      card = rendered.at_css("a[href='/schedules']")

      expect(card.at_css("path[d='#{active_schedules_icon_path}']")).to be_present
    end

    it 'renders the compliance icon on the compliance stat card' do
      rendered = render_inline(dashboard_view)
      card = rendered.at_css("a[href='/reports']")

      expect(card.at_css("path[d='#{compliance_icon_path}']")).to be_present
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

    it 'renders the reports insights link with visible link affordance' do
      presenter = person_task_presenter(insight_result: detected_insight_result, can_view_reports: true)

      rendered = render_inline(described_class.new(presenter: presenter))
      link = rendered.at_css("a[href='/reports#insights']")

      expect(link.text).to include('View Full Report')
      expect(link['class']).to include('border-b-2')
      expect(link['class']).to include('hover:border-current')
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
  end

  describe 'dashboard density' do
    it 'renders top metric cards with the compact layout' do
      rendered = render_inline(dashboard_view)
      html = rendered.to_html

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
      compliance_percentage: 85,
      next_dose_time: nil,
      smart_insights: insight_result || learning_insight_result,
      can_view_reports?: can_view_reports,
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
      status: status
    }
  end

  def as_needed_dashboard_row(person)
    {
      person: person,
      source: schedules(:john_paracetamol),
      scheduled_at: Time.current,
      taken_at: nil,
      status: :available
    }
  end
end
