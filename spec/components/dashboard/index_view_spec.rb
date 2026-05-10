# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::IndexView, type: :component do
  fixtures :accounts, :account_otp_keys, :people, :users, :locations, :medications, :dosages, :schedules,
           :person_medications, :medication_takes

  subject(:dashboard_view) do
    described_class.new(presenter: presenter)
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

  it 'renders the medicine list heading' do
    rendered = render_inline(dashboard_view)
    expect(rendered.text).to include('Today’s medicines')
  end

  describe 'daily summary' do
    it 'renders one compact daily summary with progress and next dose copy' do
      rendered = render_inline(dashboard_view)

      summary = rendered.at_css('[data-testid="dashboard-daily-summary"]')

      expect(summary).to be_present
      expect(rendered.css('[data-testid="dashboard-daily-summary"]').size).to eq(1)
      expect(summary.text).to include('Today:')
      expect(summary.text).to include('complete')
      expect(summary.text).to include('Next due')
    end

    it 'renders the daily summary above the medicine list' do
      rendered = render_inline(dashboard_view)
      summary = rendered.at_css('[data-testid="dashboard-daily-summary"]')
      medicine_list = rendered.at_css('[data-testid="dashboard-medicine-list"]')

      expect(rendered.to_html.index(summary.to_html)).to be < rendered.to_html.index(medicine_list.to_html)
    end

    it 'does not render the old multi-card analytics grid' do
      rendered = render_inline(dashboard_view)

      expect(rendered.text).not_to include('Active Schedules')
      expect(rendered.text).not_to include('Compliance')
      expect(rendered.text).not_to include('People')
      expect(rendered.css('[data-testid="dashboard-stat-grid"]')).to be_empty
    end

    it 'does not render inventory or insight panels above the medicine list' do
      rendered = render_inline(dashboard_view)

      expect(rendered.text).not_to include('Smart Insights')
      expect(rendered.text).not_to include('Stock Inventory')
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

  describe 'medicine card stack' do
    it 'renders dashboard filters with counts' do
      rendered = render_inline(dashboard_view)
      filter_strip = rendered.at_css('[data-testid="dashboard-filter-strip"]')

      expect(filter_strip).to be_present
      expect(filter_strip.text).to include('All', 'Needs action', 'Upcoming', 'Taken')
      expect(filter_strip.at_css('[aria-current="page"]').text).to include('All')
    end

    it 'filters the medicine stack to action-needed doses' do
      filtered_view = described_class.new(presenter: presenter, filter: 'needs_action')
      rendered = render_inline(filtered_view)
      cards = rendered.css('[data-testid^="dashboard-medicine-card-"]')
      medicine_rows = presenter.doses + presenter.as_needed_by_person.values.flatten
      action_needed_count = medicine_rows.count { |dose| %i[upcoming available].include?(dose[:status]) }
      active_filter = rendered.at_css('[data-testid="dashboard-filter-strip"] [aria-current="page"]')

      expect(cards.size).to eq(action_needed_count)
      expect(active_filter.text).to include('Needs action')
    end

    it 'renders medicine cards with compact metadata' do
      rendered = render_inline(dashboard_view)
      cards = rendered.css('[data-testid^="dashboard-medicine-card-"]')

      expect(cards).not_to be_empty
      expect(cards.first.text).to match(/.+ · .+ · Home/)
    end

    it 'renders the mobile flow as summary before medicines' do
      rendered = render_inline(dashboard_view)
      html = rendered.to_html

      expect(html.index('dashboard-daily-summary')).to be < html.index('dashboard-medicine-list')
    end
  end

  describe 'as-needed medicine rows' do
    it 'renders routine and as-needed medicines as redesigned cards' do
      rendered = render_inline(described_class.new(presenter: person_task_presenter))

      expect(rendered.text).to include('Vitamin D')
      expect(rendered.text).to include('Paracetamol')
      expect(rendered.css('[data-testid^="dashboard-medicine-card-"]').size).to eq(2)
      expect(rendered.at_css('[data-status="available"]')).to be_present
    end

    it 'renders as-needed availability as an actionable card' do
      rendered = render_inline(described_class.new(presenter: person_task_presenter))
      available_card = rendered.at_css('[data-status="available"]')

      expect(available_card.text).to include('Paracetamol')
      expect(available_card.text).to include('Available now')
      expect(available_card.text).to include('Give')
    end

    it 'renders blocked routine rows without an action button' do
      rendered = render_inline(described_class.new(presenter: person_task_presenter(routine_status: :out_of_stock)))
      blocked_card = rendered.css('[data-testid^="dashboard-medicine-card-"]').find do |card|
        card.text.include?('Vitamin D')
      end

      expect(blocked_card.text).to include('Vitamin D')
      expect(blocked_card.text).to include('Out of Stock')
      expect(blocked_card.text).not_to include('Give')
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

  def person_task_presenter(routine_status: :upcoming)
    person = people(:john)
    instance_double(
      DashboardPresenter,
      current_user: admin_user,
      doses: [routine_dashboard_row(person, status: routine_status)],
      next_dose_time: nil,
      as_needed_by_person: { person => [as_needed_dashboard_row(person)] }
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
