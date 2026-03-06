# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Appearance mode' do
  fixtures :accounts, :people, :users

  let(:user) { users(:damacus) }
  let(:page_background_script) do
    'getComputedStyle(document.body).backgroundColor'
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
