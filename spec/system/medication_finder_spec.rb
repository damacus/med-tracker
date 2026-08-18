# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'MedicationFinder' do
  fixtures :accounts, :people, :users, :locations, :medications

  let(:user) { users(:john) }

  before do
    driven_by(:rack_test)
    login_as(user)
  end

  it 'displays the medication finder page' do
    visit medication_finder_path

    within '[data-testid="medication-finder"]' do
      aggregate_failures 'medication finder content' do
        expect(page).to have_text('Medication Finder')
        expect(page).to have_field('medication-search-input')
        expect(page).to have_select('medication-form-filter')
        expect(page).to have_field('medication-strength-filter')
        expect(page).to have_button('Search')
        expected_text = 'Search NHS dm+d and supported product sources to add medication or restock existing inventory.'
        expect(page).to have_text(expected_text)
        expect(page).to have_css('[data-testid="barcode-scanner"]')
      end
    end
  end

  it 'keeps the search field and action readable across viewport sizes', :browser do
    driven_by(:playwright)
    page.current_window.resize_to(390, 844)
    login_as(user)

    visit medication_finder_path

    geometry = page.evaluate_script(<<~JS)
      (() => {
        const inputElement = document.querySelector('#medication-search-input');
        const input = inputElement.getBoundingClientRect();
        const button = document.querySelector('[data-medication-search-target="submitButton"]').getBoundingClientRect();
        const styles = getComputedStyle(inputElement);
        const context = document.createElement('canvas').getContext('2d');
        context.font = styles.font;
        return {
          inputRight: input.right,
          inputBottom: input.bottom,
          buttonLeft: button.left,
          buttonRight: button.right,
          buttonTop: button.top,
          placeholderWidth: context.measureText(inputElement.placeholder).width,
          usableInputWidth: inputElement.clientWidth - parseFloat(styles.paddingLeft) - parseFloat(styles.paddingRight),
          viewportWidth: window.innerWidth,
          overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth
        };
      })()
    JS

    expect(geometry.fetch('overflow')).to be <= 1
    expect(geometry.fetch('inputRight')).to be <= geometry.fetch('viewportWidth')
    expect(geometry.fetch('buttonRight')).to be <= geometry.fetch('viewportWidth')
    expect(geometry.fetch('buttonTop')).to be >= geometry.fetch('inputBottom')
    expect(geometry.fetch('placeholderWidth')).to be <= geometry.fetch('usableInputWidth')
    expect(find_field('medication-search-input')[:placeholder])
      .to eq(I18n.t('medications.finder.placeholder'))

    page.current_window.resize_to(1400, 1000)
    desktop_geometry = page.evaluate_script(<<~JS)
      (() => {
        const inputElement = document.querySelector('#medication-search-input');
        const icon = inputElement.previousElementSibling.getBoundingClientRect();
        return {
          iconWidth: icon.width,
          inputPaddingLeft: parseFloat(getComputedStyle(inputElement).paddingLeft)
        };
      })()
    JS

    expect(desktop_geometry.fetch('inputPaddingLeft')).to be >= desktop_geometry.fetch('iconWidth')
  end

  it 'opens a restock confirmation modal for an existing medication result', :browser do
    driven_by(:playwright)
    login_as(user)
    medication = medications(:vitamin_c)
    medication.update!(current_supply: 10, supply_at_last_restock: 10)
    stub_medication_finder_search(medication)

    visit medication_finder_path
    fill_in 'medication-search-input', with: 'wellman'
    click_on 'Search'
    click_on 'Update stock'

    expect(page).to have_css('[data-testid="finder-restock-modal"]')
    expect(page).to have_text("Confirm you wish to add 30 units to #{medication.display_name}.")

    click_on 'Confirm restock'

    expect(page).to have_text('Inventory refilled successfully.')
    expect(medication.reload.current_supply).to eq(40)
  end

  it 'expands structured details for an external medicine result', :browser do
    driven_by(:playwright)
    login_as(user)
    stub_medication_finder_payload(
      results: [
        {
          name: 'Aspirin 300mg tablets',
          display: 'Aspirin 300mg tablets',
          description: 'Pain relief medicine',
          directions: 'Take with water',
          warnings: 'Do not exceed the stated dose',
          category: 'Analgesic',
          package_size: '32 tablets',
          source_label: 'NHS dm+d'
        }
      ],
      permissions: { can_create: true, can_restock: true }
    )

    visit medication_finder_path
    fill_in 'medication-search-input', with: 'aspirin'
    click_on 'Search'
    click_on 'View medicine details'

    expect(page).to have_css('[data-testid="medicine-details"]')
    expect(page).to have_text('Pain relief medicine')
    expect(page).to have_text('Take with water')
    expect(page).to have_text('Do not exceed the stated dose')
  end

  it 'sends the selected strength with a search', :browser do
    driven_by(:playwright)
    login_as(user)
    stub_medication_finder_payload(results: [], permissions: { can_create: true, can_restock: true })

    visit medication_finder_path
    fill_in 'medication-search-input', with: 'paracetamol'
    fill_in 'medication-strength-filter', with: '500mg'
    click_on 'Search'

    request_url = page.driver.with_playwright_page do |playwright_page|
      playwright_page.evaluate('window.medicationFinderRequestUrl')
    end
    expect(request_url).to include('q=paracetamol', 'strength=500mg')
  end

  it 'shows when lower-confidence review items were filtered', :browser do
    driven_by(:playwright)
    login_as(user)
    stub_medication_finder_payload(
      results: [
        {
          name: 'Example medicine',
          display: 'Example medicine',
          review_prompts: [],
          review_prompt_filter: { hidden_count: 12 }
        }
      ],
      permissions: { can_create: true, can_restock: true }
    )

    visit medication_finder_path
    fill_in 'medication-search-input', with: 'example'
    click_on 'Search'

    expect(page).to have_css('[data-testid="filtered-review-prompts"]')
    expect(page).to have_text('12 lower-confidence review items hidden to reduce noise')
  end

  it 'links to related household medicines without offering to restock them', :browser do
    driven_by(:playwright)
    page.current_window.resize_to(390, 844)
    login_as(user)
    related_medication = medications(:vitamin_c)
    stub_medication_finder_payload(
      results: [
        {
          name: 'Related medicine pack',
          display: 'Related medicine pack',
          related_medications_html: Components::Medications::RelatedMedicationsPrompt.new(
            medications: [
              {
                id: related_medication.id,
                name: related_medication.display_name,
                location: related_medication.location.name,
                path: medication_path(related_medication),
                current_supply: '10'
              }
            ],
            heading_id: 'related-medications-0'
          ).call
        }
      ],
      permissions: { can_create: true, can_restock: true }
    )

    visit medication_finder_path
    fill_in 'medication-search-input', with: 'related medicine'
    click_on 'Search'

    expect(page).to have_css('aside[data-testid="related-medications-prompt"][aria-labelledby]')
    within 'aside[data-testid="related-medications-prompt"]' do
      expect(page).to have_css('h3[id^="related-medications-"]', text: 'Related medicine in your household')
      expect(page).to have_link(related_medication.display_name, href: medication_path(related_medication))
      expect(page).to have_css('dl > dt', text: 'Location')
      expect(page).to have_css('dl > dd', text: related_medication.location.name)
      expect(page).to have_css('dl > dt', text: 'Current supply')
      expect(page).to have_css('dl > dd', text: '10')
    end
    expect(page).to have_no_button('Update stock')
    expect(page.evaluate_script('document.documentElement.scrollWidth')).to be <= 390
  end

  def stub_medication_finder_search(medication)
    stub_medication_finder_payload(**medication_finder_payload(medication))
  end

  def stub_medication_finder_payload(payload = nil, **overrides)
    payload ||= overrides
    page.driver.with_playwright_page do |playwright_page|
      playwright_page.add_init_script(
        script: "window.medicationFinderPayload = #{payload.to_json};" \
                'window.fetch = async (url) => {' \
                'window.medicationFinderRequestUrl = String(url);' \
                'return { ok: true, json: async () => window.medicationFinderPayload };' \
                '};'
      )
    end
  end

  def medication_finder_payload(medication)
    {
      results: [
        medication_finder_result(medication)
      ],
      permissions: { can_create: true, can_restock: true },
      barcode: '5021265221301'
    }
  end

  def medication_finder_result(medication)
    {
      name: 'Wellman Original',
      display: 'Wellman Original (Vitabiotics) 30 tablets',
      source_label: 'Open Food Facts',
      package_size: '30 tablets',
      package_quantity: 30,
      package_unit: 'tablet',
      existing_medication: medication_finder_existing_medication(medication)
    }
  end

  def medication_finder_existing_medication(medication)
    {
      id: medication.id,
      name: medication.display_name,
      location: medication.location.name,
      path: medication_path(medication),
      refill_path: refill_medication_path(medication),
      current_supply: '10'
    }
  end
end
