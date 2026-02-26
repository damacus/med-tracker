# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::IndexView, type: :component do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules,
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
  end

  describe 'quick actions' do
    it 'renders Add Medication and Add Person links' do
      rendered = render_inline(dashboard_view)
      expect(rendered.text).to include('Add Medication')
      expect(rendered.text).to include('Add Person')
    end
  end

  describe 'high-fidelity sections' do
    it 'renders Smart Insights' do
      rendered = render_inline(dashboard_view)
      expect(rendered.text).to include('Smart Insights')
    end

    it 'renders Stock Inventory' do
      rendered = render_inline(dashboard_view)
      expect(rendered.text).to include('Stock Inventory')
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
end
