# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Reports::Index do
  subject(:report_view) do
    described_class.new(
      daily_data: daily_data,
      inventory_alerts: inventory_alerts,
      start_date: 7.days.ago.to_date,
      end_date: Time.zone.today
    )
  end

  let(:daily_data) do
    [
      { day_name: 'Mon', percentage: 100, expected: 5, actual: 5 },
      { day_name: 'Tue', percentage: 90, expected: 10, actual: 9 },
      { day_name: 'Wed', percentage: 80, expected: 5, actual: 4 }
    ]
  end

  let(:inventory_alerts) do
    [
      { medication_name: 'Ibuprofen', days_left: 3, doses_left: 12 }
    ]
  end

  before do
    # rubocop:disable RSpec/SubjectStub
    helper_view_context = controller.view_context
    allow(helper_view_context).to receive(:reports_path).and_return('/reports')
    allow(report_view).to receive(:view_context).and_return(helper_view_context)
    # rubocop:enable RSpec/SubjectStub
  end

  it 'renders the health report heading' do
    rendered = render report_view
    expect(rendered).to include('Health Report')
    expect(rendered).to include('Wellness Analytics')
  end

  it 'renders the compliance stats' do
    rendered = render report_view
    expect(rendered).to include('Overall Compliance')
    expect(rendered).to include('Total Doses Logged')
  end

  it 'renders the adherence timeline' do
    rendered = render report_view
    expect(rendered).to include('Adherence Timeline')
    expect(rendered).to include('Mon')
    expect(rendered).to include('Tue')
    expect(rendered).to include('Wed')
  end

  it 'renders the inventory alert with interpolated data' do
    rendered = render report_view
    expect(rendered).to include('Inventory Alert')
    expect(rendered).to include('Ibuprofen')
    expect(rendered).to include('exhausted in 3 days')
  end

  it 'renders report copy from the active locale' do
    rendered = I18n.with_locale(:ga) { render report_view }

    expect(rendered).to include(I18n.t('reports.index.title', locale: :ga))
    expect(rendered).to include(I18n.t('reports.index.timeline_title', locale: :ga))
  end

  it 'uses token-driven glow surfaces instead of hard-coded analytics gradients' do
    rendered = render report_view

    glow_tokens = rendered.scan(/report-(?:hero-surface|glow-panel|glow-orb)/).uniq
    banned_tokens = ['bg-indigo-300', 'bg-success-light', 'bg-destructive-light', 'from-indigo-600 to-violet-700']

    expect(glow_tokens).to include('report-hero-surface', 'report-glow-panel', 'report-glow-orb')
    expect(banned_tokens.any? { |token| rendered.include?(token) }).to be(false)
  end
end
