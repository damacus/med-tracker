require 'rails_helper'

RSpec.describe 'Report exports', :browser do
  fixtures :all

  before do
    driven_by(:playwright)
    login_as(users(:admin))
  end

  it 'keeps the GP report selector within the mobile viewport and preserves the review export panel' do
    page.current_window.resize_to(390, 844)
    visit reports_path

    expect(page).to have_select('Person')
    expect(page).to have_button('Download PDF')
    expect(page_horizontal_overflow).to be <= 1

    visit medication_review_prompts_path(household_slug: browser_household.slug)

    expect(page).to have_text('The PDF contains the default visible medicine review scope.')
    expect(page).to have_css('[data-testid="pdf-export-panel"]')
    expect(page_horizontal_overflow).to be <= 1
  end

  it 'shows the GP export controls at desktop and mobile widths' do
    [[1440, 1000], [390, 844]].each do |width, height|
      page.current_window.resize_to(width, height)
      visit reports_path
      scroll_to_gp_export_controls

      expect(page).to have_select('Person for GP report', visible: :visible)
      expect(page).to have_unchecked_field('Include medication administration log', visible: :visible)
      expect(page).to have_button('Download PDF', visible: :visible)
      expect(page_horizontal_overflow).to be <= 1
    end
  end

  it 'shows renderer feedback after a GP selector submission fails' do
    visit reports_path

    renderer = instance_double(Reports::HealthHistoryPdf)
    allow(Reports::HealthHistoryPdf).to receive(:new).and_return(renderer)
    allow(renderer).to receive(:render).and_raise(Reports::PdfRenderer::Error, 'renderer failed')

    select people(:john).name, from: 'Person for GP report'
    check 'Include medication administration log'
    click_button 'Download PDF'

    expect(page).to have_text(I18n.t('reports.export.pdf_unavailable'))
    expect(page).to have_current_path(reports_path(person_id: people(:john).id, include_medication_takes: '1'))
    expect(page).to have_checked_field('Include medication administration log')
  end

  def page_horizontal_overflow
    page.evaluate_script(<<~JS)
      (() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - document.documentElement.clientWidth)()
    JS
  end

  def scroll_to_gp_export_controls
    page.execute_script("document.getElementById('health_history_person_id').scrollIntoView({ block: 'center' })")
  end
end
