# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Offline dose eligibility', :browser do
  fixtures :accounts, :people, :users

  before do
    login_as(users(:admin))
    page.driver.with_playwright_page do |browser|
      browser.add_init_script(script: "Object.defineProperty(navigator, 'onLine', { get: () => false });")
    end
    visit offline_path
  end

  def cache_dose(script = '')
    expect(page).to have_css('[data-offline-shell-target="connection"]', text: 'Offline')
    page.driver.with_playwright_page { |browser| browser.evaluate(dose_script(script)) }
    expect(page).to have_text('Test medicine')
    settle_layout
  end

  def dose_script(script)
    <<~JS
      async () => {
        const store = await import('controllers/offline_store');
        const medication = { id: 501, name: 'Test medicine', dose_amount: 1, dose_unit: 'tablet', current_supply: 10 };
        const eligibility = { allowed: true, valid_until: new Date(Date.now() + 60000).toISOString(), dose_amount: '1', dose_unit: 'tablet' };
        const source = { id: 901, person_id: 801, medication_id: 501, dose_amount: 1, dose_unit: 'tablet', offline_eligibility: eligibility };
        #{script}
        await store.saveSnapshot({ data: { medications: [medication], people: [{ id: 801, name: 'Test person' }], schedules: [source] } });
        window.dispatchEvent(new CustomEvent('medtracker:offline-take-queued'));
      }
    JS
  end

  def settle_layout
    page.driver.with_playwright_page do |browser|
      browser.evaluate(<<~JS)
        async () => {
          await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
          await Promise.all(document.getAnimations().filter(animation => animation.effect.getTiming().iterations !== Infinity).map(animation => animation.finished.catch(() => {})));
        }
      JS
    end
  end

  {
    'restricted' => ["eligibility.allowed = false; eligibility.reason = 'Paused';", 'Paused'],
    'stale' => ["eligibility.valid_until = '2000-01-01T00:00:00Z';", 'Refresh your care plan'],
    'missing eligibility' => ['delete source.offline_eligibility;', 'Refresh your care plan']
  }.each do |scenario, (script, reason)|
    it "disables #{scenario} cached doses with a visible reason" do
      cache_dose(script)
      expect(page).to have_button('Unavailable', disabled: true)
      expect(page).to have_text(reason)
      expect(page).to have_css('[data-offline-shell-target="pendingCount"]', text: '0')
    end
  end

  it 'queues an eligible dose and prevents another until it syncs' do
    cache_dose
    click_button 'Take now'
    expect(page).to have_css('[data-offline-shell-target="pendingCount"]', text: '1')
    expect(page).to have_button('Unavailable', disabled: true)
    expect(page).to have_text('Sync the pending dose before recording another dose')
  end

  %w[tablet capsule gummy sachet spray drop pad ml].each do |unit|
    it "requires enough stock for the full #{unit} dose" do
      cache_dose(<<~JS)
        medication.current_supply = 1;
        medication.dose_unit = '#{unit}';
        eligibility.dose_amount = '2';
        eligibility.dose_unit = '#{unit}';
      JS
      expect(page).to have_button('Out of stock', disabled: true)
    end
  end

  it 'deducts all units of a pending dose from the displayed inventory' do
    cache_dose("medication.current_supply = 3; eligibility.dose_amount = '2';")
    click_button 'Take now'
    expect(page).to have_css('[data-offline-shell-target="pendingCount"]', text: '1')
    expect(page).to have_button('Out of stock', disabled: true)
    within '[data-offline-shell-target="people"]' do
      expect(page).to have_text('Test medicine 1', normalize_ws: true)
    end
  end
end
