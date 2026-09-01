require 'rails_helper'

RSpec.describe Components::Reports::ExportPanel, type: :component do
  subject(:panel) do
    described_class.new(
      href: '/reports/health-history?person_id=1',
      title: 'Export health history',
      description: 'Download a PDF of the people and dates currently shown.',
      scope: 'People: John Doe. Date range: 1 January 2026 to 31 January 2026.',
      label: 'Download PDF',
      preparing_label: 'Preparing PDF...'
    )
  end

  it 'renders a responsive export control with the active scope' do
    rendered = render_inline(panel)
    section = rendered.at_css('[data-testid="pdf-export-panel"]')

    expect(section['class']).to include('flex-col', 'sm:flex-row')
    expect(section.text).to include('Export health history', 'People: John Doe')
  end

  it 'configures the export control with safe initial state' do
    link = export_link

    expect(link['data-controller']).to include('pdf-export')
    expect(link['data-action']).to include('click->pdf-export#prepare')
    expect(link['aria-busy']).to eq('false')
    expect(link['aria-disabled']).to eq('false')
  end

  it 'announces PDF preparation through the accessible label' do
    link = export_link

    expect(link['data-pdf-export-preparing-label-value']).to eq('Preparing PDF...')
    expect(link.at_css('[data-pdf-export-target="label"]')['aria-live']).to eq('polite')
  end

  def export_link
    render_inline(panel).at_css('a[href="/reports/health-history?person_id=1"]')
  end
end
