require 'rails_helper'

RSpec.describe Components::Reports::PdfDocument, type: :component do
  it 'renders a complete A4 document with shared report furniture and pagination rules', :aggregate_failures do
    component = described_class.new(
      title: 'Medicine review record',
      context: 'Alex Smith · 1 September 2026',
      generated_at: Time.utc(2026, 9, 1, 10, 30),
      content: Class.new(Phlex::HTML) {
        def view_template
          section(class: 'report-section') do
            h2 { 'Review items' }
            div(class: 'callout') { 'Discuss this report with a practitioner.' }
            table do
              thead { tr { th { 'Medicine' } } }
              tbody { tr { td { 'Paracetamol' } } }
            end
          end
        end
      }.new
    )
    html = component.call
    rendered = Nokogiri::HTML5(html)
    stylesheet = rendered.at_css('style').text

    expect(html).to include('<html lang="en">')
    expect(rendered.at_css('header .report-brand').text).to eq('MEDTRACKER')
    expect(rendered.at_css('header h1').text).to eq('Medicine review record')
    expect(rendered.at_css('header .report-context').text).to include('Alex Smith')
    expect(rendered.at_css('header time')['datetime']).to eq('2026-09-01T10:30:00Z')
    expect(stylesheet).to include(
      '@page',
      'size: A4',
      'margin: 18mm 16mm 20mm',
      'counter(page)',
      'counter(pages)',
      'font-family: "Noto Sans"',
      'thead { display: table-header-group',
      'break-inside: avoid',
      'widows: 3',
      'orphans: 3',
      'break-after: avoid'
    )
  end
end
