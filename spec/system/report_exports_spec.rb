require 'rails_helper'

RSpec.describe 'Report exports', :browser do
  fixtures :all

  before do
    driven_by(:playwright)
    login_as(users(:admin))
  end

  it 'announces preparation for keyboard activation and resets without moving focus' do
    visit reports_path

    export = find('[data-testid="pdf-export-panel"] a')
    page.execute_script("arguments[0].setAttribute('href', '#')", export)
    export.send_keys(:enter)

    expect(export['aria-busy']).to eq('true')
    expect(export['aria-disabled']).to eq('true')
    expect(export).to have_text('Preparing PDF...')
    duplicate_prevented = page.evaluate_script(<<~JS, export)
      (() => {
        const event = new MouseEvent('click', { bubbles: true, cancelable: true });
        return !arguments[0].dispatchEvent(event);
      })()
    JS

    expect(duplicate_prevented).to be(true)

    expect(page).to have_css('[data-testid="pdf-export-panel"] a[aria-busy="false"]')
    expect(page.evaluate_script('document.activeElement === arguments[0]', export)).to be(true)
  end

  it 'keeps export panels within the mobile viewport and explains the review scope' do
    page.current_window.resize_to(390, 844)
    visit reports_path

    expect(page).to have_css('[data-testid="pdf-export-panel"]')
    expect(page_horizontal_overflow).to be <= 1

    visit medication_review_prompts_path

    expect(page).to have_text('The PDF contains the default visible medicine review scope.')
    expect(page).to have_css('[data-testid="pdf-export-panel"]')
    expect(page_horizontal_overflow).to be <= 1
  end

  def page_horizontal_overflow
    page.evaluate_script(<<~JS)
      (() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - document.documentElement.clientWidth)()
    JS
  end
end
