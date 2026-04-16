# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Appearance mode' do
  fixtures :accounts, :people, :users

  let(:user) { users(:damacus) }
  let(:page_background_script) do
    'getComputedStyle(document.body).backgroundColor'
  end

  let(:root_primary_script) do
    'getComputedStyle(document.documentElement).getPropertyValue("--primary").trim().toLowerCase()'
  end

  it 'lets signed-in users switch appearance modes without losing their palette' do
    sign_in(user)
    visit profile_path

    click_button 'Dark'

    default_dark_background = page.evaluate_script(page_background_script)

    expect(page.evaluate_script('localStorage.getItem("med-tracker-appearance")')).to eq('dark')
    expect(page.evaluate_script('document.documentElement.classList.contains("dark")')).to be(true)

    click_button 'Warm Earth'

    expect(page.evaluate_script(page_background_script)).not_to eq(default_dark_background)
    expect(page.evaluate_script('localStorage.getItem("med-tracker-theme")')).to eq('warm-earth')
    expect(page.evaluate_script('document.documentElement.classList.contains("dark")')).to be(true)
    expect(page.evaluate_script('document.documentElement.classList.contains("theme-warm-earth")')).to be(true)
  end

  it 'applies palette changes from profile to medication surfaces in the same session' do
    medication = create(:medication, location: create(:location), name: 'Theme Proof')

    sign_in(user)
    visit medication_path(medication)

    default_primary = page.evaluate_script(root_primary_script)
    default_medication_icon_background = page.evaluate_script(<<~JS)
      (() => {
        const icon = document.querySelector('[data-testid="medication-hero-icon"]')
        return icon ? getComputedStyle(icon).backgroundColor : null
      })()
    JS

    visit profile_path

    default_profile_card_background = page.evaluate_script(<<~JS)
      (() => {
        const card = document.querySelector('[data-testid="profile-personal-info-card"]')
        return card ? getComputedStyle(card).backgroundColor : null
      })()
    JS

    expect(default_medication_icon_background).not_to be_nil
    expect(default_profile_card_background).not_to be_nil

    click_button 'Warm Earth'

    warm_profile_primary = page.evaluate_script(root_primary_script)

    warm_profile_card_background = page.evaluate_script(<<~JS)
      (() => {
        const card = document.querySelector('[data-testid="profile-personal-info-card"]')
        return card ? getComputedStyle(card).backgroundColor : null
      })()
    JS

    expect(warm_profile_primary).not_to eq(default_primary)
    expect(warm_profile_card_background).not_to eq(default_profile_card_background)

    visit medication_path(medication)

    warm_medication_primary = page.evaluate_script(root_primary_script)

    warm_medication_icon_background = page.evaluate_script(<<~JS)
      (() => {
        const icon = document.querySelector('[data-testid="medication-hero-icon"]')
        return icon ? getComputedStyle(icon).backgroundColor : null
      })()
    JS

    expect(warm_medication_primary).to eq(warm_profile_primary)
    expect(warm_medication_icon_background).not_to eq(default_medication_icon_background)
  end

  it 'honors system dark appearance on signed-out pages' do
    visit login_path

    page.driver.with_playwright_page do |playwright_page|
      playwright_page.add_init_script(script: <<~JS)
        const originalMatchMedia = window.matchMedia.bind(window)
        window.matchMedia = (query) => {
          if (query === "(prefers-color-scheme: dark)") {
            return {
              matches: true,
              media: query,
              onchange: null,
              addEventListener: () => {},
              removeEventListener: () => {},
              addListener: () => {},
              removeListener: () => {},
              dispatchEvent: () => true
            }
          }

          return originalMatchMedia(query)
        }
      JS
    end

    page.execute_script('localStorage.setItem("med-tracker-appearance", "system")')
    visit login_path

    expect(page.evaluate_script('document.documentElement.dataset.appearance')).to eq('system')
    expect(page.evaluate_script('document.documentElement.classList.contains("dark")')).to be(true)
    expect(page.evaluate_script('document.querySelector(\'meta[name="theme-color"]\').content')).to eq('#111827')
  end

  it 'preserves the saved palette on signed-out pages without applying it' do
    visit login_path
    page.execute_script('localStorage.setItem("med-tracker-theme", "warm-earth")')

    visit login_path

    expect(page.evaluate_script('localStorage.getItem("med-tracker-theme")')).to eq('warm-earth')
    expect(page.evaluate_script('document.documentElement.classList.contains("theme-warm-earth")')).to be(false)
  end
end
