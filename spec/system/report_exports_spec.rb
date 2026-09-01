require 'rails_helper'

RSpec.describe 'Report exports', :browser do
  fixtures :all

  before do
    driven_by(:playwright)
    login_as(users(:admin))
  end

  it 'keeps keyboard activation busy until a delayed PDF download completes without moving focus' do
    visit reports_path

    export = find('[data-testid="pdf-export-panel"] a')
    defer_pdf_request(export)
    export.send_keys(:enter)

    expect(export['aria-busy']).to eq('true')
    expect(export['aria-disabled']).to eq('true')
    expect(export).to have_text('Preparing PDF...')
    page.execute_script("window.setTimeout(() => { document.body.dataset.pdfExportDelayElapsed = 'true' }, 1100)")
    expect(page).to have_css('body[data-pdf-export-delay-elapsed="true"]')
    expect(export['aria-busy']).to eq('true')
    duplicate_prevented = page.evaluate_script(<<~JS, export)
      (() => {
        const event = new MouseEvent('click', { bubbles: true, cancelable: true });
        return !arguments[0].dispatchEvent(event);
      })()
    JS

    expect(duplicate_prevented).to be(true)

    resolve_pdf_request

    expect(page).to have_css('[data-testid="pdf-export-panel"] a[aria-busy="false"]')
    expect(page.evaluate_script('document.activeElement === arguments[0]', export)).to be(true)
  end

  it 'resets an in-progress export when Turbo caches the document' do
    visit reports_path

    export = find('[data-testid="pdf-export-panel"] a')
    defer_pdf_request(export)
    defer_export_controller_lookup
    start_export(export)

    expect(export['aria-busy']).to eq('true')
    page.execute_script("document.dispatchEvent(new Event('turbo:before-cache'))")

    expect(export['aria-busy']).to eq('false')
    expect(export['aria-disabled']).to eq('false')
    expect(export).to have_text('Download PDF')
  end

  it 'aborts and ignores a deferred response after Turbo caches the document' do
    visit reports_path

    export = find('[data-testid="pdf-export-panel"] a')
    defer_pdf_request(export)
    start_export(export)

    page.execute_script("document.dispatchEvent(new Event('turbo:before-cache'))")

    expect(page.evaluate_script('window.pdfExportAborted')).to be(true)
    resolve_pdf_request

    expect(page.evaluate_script('window.pdfExportDownloads')).to eq(0)
    expect(export['aria-busy']).to eq('false')
  end

  it 'follows the HTML fallback without retrying a redirected PDF export' do
    visit reports_path

    export = find('[data-testid="pdf-export-panel"] a')
    fallback_location = export['data-pdf-export-fallback-location-value']
    export_location = export[:href]
    stub_redirected_pdf_response(export_location)
    start_export(export)

    expect(page).to have_current_path(fallback_location)
    expect(page.evaluate_script("window.sessionStorage.getItem('pdfExportFetchCount')")).to eq('1')
    expect(page).to have_no_current_path(export_location)
  end

  it 'keeps export panels within the mobile viewport and explains the review scope' do
    page.current_window.resize_to(390, 844)
    visit reports_path

    expect(page).to have_css('[data-testid="pdf-export-panel"]')
    expect(page_horizontal_overflow).to be <= 1

    visit medication_review_prompts_path(household_slug: browser_household.slug)

    expect(page).to have_text('The PDF contains the default visible medicine review scope.')
    expect(page).to have_css('[data-testid="pdf-export-panel"]')
    expect(page_horizontal_overflow).to be <= 1
  end

  def page_horizontal_overflow
    page.evaluate_script(<<~JS)
      (() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - document.documentElement.clientWidth)()
    JS
  end

  def defer_pdf_request(export)
    track_pdf_downloads
    page.execute_script(<<~JS, export)
      window.fetch = (_url, options) => new Promise((resolve) => {
        options?.signal?.addEventListener('abort', () => {
          window.pdfExportAborted = true;
        });
        window.resolvePdfExport = resolve;
      });
      arguments[0].focus();
    JS
  end

  def track_pdf_downloads
    page.execute_script(<<~JS)
      window.pdfExportAborted = false;
      window.pdfExportDownloads = 0;
      window.pdfExportCreateObjectUrl = URL.createObjectURL;
      URL.createObjectURL = (blob) => {
        window.pdfExportDownloads += 1;
        return window.pdfExportCreateObjectUrl(blob);
      };
    JS
  end

  def resolve_pdf_request
    page.execute_script(<<~JS)
      window.resolvePdfExport(
        new Response(new Blob(['%PDF-1.7'], { type: 'application/pdf' }), {
          status: 200,
          headers: {
            'Content-Type': 'application/pdf',
            'Content-Disposition': 'attachment; filename="health-history.pdf"'
          }
        })
      );
    JS
  end

  def defer_export_controller_lookup
    page.execute_script(<<~JS)
      const stimulus = window.Stimulus;
      const lookup = stimulus.getControllerForElementAndIdentifier.bind(stimulus);

      stimulus.getControllerForElementAndIdentifier = () => {
        stimulus.getControllerForElementAndIdentifier = lookup;
        return null;
      };
    JS
  end

  def stub_redirected_pdf_response(export_location)
    page.execute_script(redirected_response_script, export_location)
  end

  def redirected_response_script
    <<~JS
      window.sessionStorage.setItem('pdfExportFetchCount', '0');
      const exportLocation = new URL(arguments[0], window.location.href);
      const originalFetch = window.fetch.bind(window);
      window.fetch = (url, options) => {
        const requestLocation = new URL(url, window.location.href);
        if (requestLocation.href !== exportLocation.href) return originalFetch(url, options);

        const count = Number(window.sessionStorage.getItem('pdfExportFetchCount')) + 1;
        window.sessionStorage.setItem('pdfExportFetchCount', String(count));
        const response = new Response('<html></html>', {
          status: 200,
          headers: { 'Content-Type': 'text/html' }
        });
        Object.defineProperty(response, 'redirected', { value: true });
        return Promise.resolve(response);
      };
    JS
  end

  def start_export(export)
    connected_export = find('[data-testid="pdf-export-panel"] a') do |candidate|
      page.evaluate_script(<<~JS, candidate, export)
        arguments[0] === arguments[1] &&
          Boolean(window.Stimulus.getControllerForElementAndIdentifier(arguments[0], 'pdf-export'));
      JS
    end

    page.execute_script(<<~JS, connected_export)
      window.Stimulus.getControllerForElementAndIdentifier(arguments[0], 'pdf-export').prepare(
        new Event('click', { cancelable: true })
      );
    JS
  end
end
