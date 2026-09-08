# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Offline stock selection', :browser do
  fixtures :accounts, :people, :users

  before do
    login_as(users(:admin))
    page.driver.with_playwright_page do |browser|
      browser.add_init_script(script: "Object.defineProperty(navigator, 'onLine', { get: () => false });")
    end
    visit offline_path
  end

  def seed_snapshot(script = '')
    expect(page).to have_css('[data-offline-shell-target="connection"]', text: 'Offline')
    page.driver.with_playwright_page { |browser| browser.evaluate(snapshot_script(script)) }
    expect(page).to have_text('First medicine')
    settle_layout
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

  def snapshot_script(script)
    <<~JS
      async () => {
        const store = await import('controllers/offline_store');
        const medication = { id: 501, name: 'First medicine', dose_amount: 1, dose_unit: 'tablet', current_supply: 1 };
        const eligibility = { allowed: true, valid_until: new Date(Date.now() + 60000).toISOString(), dose_amount: '1', dose_unit: 'tablet' };
        const source = { id: 901, person_id: 801, medication_id: 501, dose_amount: 1, dose_unit: 'tablet', offline_eligibility: eligibility };
        const payload = { data: { people: [{ id: 801, name: 'Test person' }], medications: [medication, { ...medication, id: 502 }], schedules: [source] } };
        #{script}
        await store.saveSnapshot(payload);
        window.dispatchEvent(new CustomEvent('medtracker:offline-take-queued'));
      }
    JS
  end

  it 'selects remaining stock through the rendered Take now button' do
    seed_snapshot(<<~JS)
      await store.queueTake({ source_type: 'schedule', source_id: 900, taken_from_medication_id: 501, dose_amount: 1, dose_unit: 'tablet' });
    JS
    click_button 'Take now'
    expect(page).to have_css('[data-offline-shell-target="pendingCount"]', text: '2')

    result = page.driver.with_playwright_page do |browser|
      browser.evaluate(<<~JS)
        async () => {
          const store = await import('controllers/offline_store');
          return (await store.getQueuedTakes()).find(take => take.source_id === 901).taken_from_medication_id;
        }
      JS
    end
    expect(result).to eq(502)
  end

  it 'keeps rapid clicks for different medicines while ignoring a duplicate click' do
    seed_snapshot(<<~JS)
      payload.data.medications[1].name = 'Second medicine';
      payload.data.schedules.push({ ...source, id: 902, medication_id: 502 });
    JS
    expect(page).to have_button('Take now', count: 2)
    page.execute_script(<<~JS)
      const buttons = document.querySelectorAll('[data-action="offline-shell#queue"]');
      buttons[0].click();
      buttons[0].click();
      buttons[1].click();
    JS
    expect(page).to have_css('[data-offline-shell-target="pendingCount"]', text: '2')
  end

  it 'serializes stock reservations from separate browser documents' do
    result = page.driver.with_playwright_page do |browser|
      browser.evaluate(<<~JS)
        async () => {
          const frame = document.createElement('iframe');
          frame.src = window.location.href;
          await new Promise(resolve => { frame.onload = resolve; document.body.append(frame); });
        }
      JS
      browser.frames.last.evaluate("async () => { window.offlineStore = await import('controllers/offline_store'); }")
      browser.evaluate(<<~JS)
        async () => {
          const frame = document.querySelector('iframe');
          const store = await import('controllers/offline_store');
          const otherStore = frame.contentWindow.offlineStore;
          const tenant = `test:${crypto.randomUUID()}`;
          const reserve = queued => queued.length ? null : { source_type: 'schedule', source_id: 901, taken_from_medication_id: 501 };
          try {
            await Promise.all([store.queueTakeIfAvailable(reserve, tenant), otherStore.queueTakeIfAvailable(reserve, tenant)]);
            return (await store.getQueuedTakes(tenant)).length;
          } finally { frame.remove(); }
        }
      JS
    end
    expect(result).to eq(1)
  end
end
