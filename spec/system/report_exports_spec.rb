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

  it 'shows renderer feedback after a GP selector submission fails' do
    visit reports_path

    renderer = instance_double(Reports::HealthHistoryPdf)
    allow(Reports::HealthHistoryPdf).to receive(:new).and_return(renderer)
    allow(renderer).to receive(:render).and_raise(Reports::PdfRenderer::Error, 'renderer failed')

    select people(:john).name, from: 'Person for GP report'
    click_button 'Download PDF'

    expect(page).to have_text(I18n.t('reports.export.pdf_unavailable'))
    expect(page).to have_current_path(reports_path(person_id: people(:john).id))
  end

  def page_horizontal_overflow
    page.evaluate_script(<<~JS)
      (() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - document.documentElement.clientWidth)()
    JS
  end
end
