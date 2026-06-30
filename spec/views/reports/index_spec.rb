# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Reports::Index do
  subject(:report_view) do
    described_class.new(
      daily_data: daily_data,
      smart_insights: smart_insights,
      start_date: 7.days.ago.to_date,
      end_date: Time.zone.today,
      today_taken_medications: today_taken_medications,
      people: people,
      selected_person_id: selected_person_id
    )
  end

  let(:today_taken_medications) do
    [
      Reports::TodayTakenMedicationsQuery::PersonGroup.new(
        person: person,
        medications: [
          Reports::TodayTakenMedicationsQuery::MedicationSummary.new(id: 1, name: 'Paracetamol')
        ]
      )
    ]
  end

  let(:daily_data) do
    [
      { day_name: 'Mon', percentage: 100, expected: 5, actual: 5 },
      { day_name: 'Tue', percentage: 90, expected: 10, actual: 9 },
      { day_name: 'Wed', percentage: 80, expected: 5, actual: 4 }
    ]
  end

  let(:smart_insights) do
    insight = SmartInsights::Insight.new(
      key: :inventory_risk,
      family: :inventory,
      severity: :urgent,
      title: 'Supply needs attention',
      summary: 'Ibuprofen has about 3 days of supply left.',
      detail: 'Check the stock record for Ibuprofen and plan the next refill.',
      metric_label: 'Supply window',
      metric_value: '3 days',
      cta_path: '/reports#insights'
    )

    SmartInsights::Result.new(
      primary_insight: insight,
      insights: [insight],
      learning_state?: false,
      evidence_summary: '12 medication events across 7 days'
    )
  end

  before do
    # rubocop:disable RSpec/SubjectStub
    helper_view_context = controller.view_context
    allow(helper_view_context).to receive(:reports_path).and_return('/reports')
    allow(report_view).to receive(:view_context).and_return(helper_view_context)
    # rubocop:enable RSpec/SubjectStub
  end

  def person
    @person ||= build_stubbed(:person, id: 1, name: 'John Doe')
  end

  def other_person
    @other_person ||= build_stubbed(:person, id: 2, name: 'Jane Doe')
  end

  def people
    [person, other_person]
  end

  def selected_person_id
    nil
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

  it 'renders compliance bars without inline styles' do
    rendered = Nokogiri::HTML.fragment(render(report_view))
    bars = rendered.css('[data-testid="report-compliance-bar"]')

    expect(bars.count).to eq(3)
    expect(bars.pluck('style')).to all(be_blank)
    expect(bars.first['class']).to include('report-compliance-bar-height-100')
    expect(bars.last['class']).to include('report-compliance-bar-height-80')
  end

  it 'renders the Today section and person filter' do
    rendered = render report_view

    expect(rendered).to include('Today')
    expect(rendered).to include('All people')
    expect(rendered).to include('John Doe')
    expect(rendered).to include('Paracetamol')
  end

  it 'does not render timestamps in the Today section' do
    rendered = render report_view

    today_section = rendered.match(%r{<section id="today".*?</section>}m).to_s
    expect(today_section).not_to include('08:00')
    expect(today_section).not_to include('noon')
  end

  context 'when no medications have been taken today' do
    let(:today_taken_medications) { [] }

    it 'renders an empty state' do
      rendered = render report_view

      expect(rendered).to include('No medications taken today.')
    end
  end

  it 'renders the smart insight with interpolated data' do
    rendered = render report_view
    expect(rendered).to include('Smart Insights')
    expect(rendered).to include('Ibuprofen')
    expect(rendered).to include('about 3 days of supply left')
  end

  it 'renders report copy from the active locale' do
    rendered = I18n.with_locale(:ga) { render report_view }

    expect(rendered).to include(I18n.t('reports.index.title', locale: :ga))
    expect(rendered).to include(I18n.t('reports.index.timeline_title', locale: :ga))
  end

  it 'uses token-driven report surfaces instead of bespoke analytics gradients and tinted cards' do
    rendered = render report_view

    expect(rendered).not_to include('bg-gradient-to-br from-indigo-600 to-violet-700')
    expect(rendered).not_to include('bg-indigo-300')
    expect(rendered).not_to include('bg-success-light')
    expect(rendered).not_to include('bg-destructive-light')
    expect(rendered).not_to include('bg-card/70')
  end
end
