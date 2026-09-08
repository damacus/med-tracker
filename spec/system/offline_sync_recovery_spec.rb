# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Offline sync recovery', :browser do
  fixtures :accounts, :people, :users

  before do
    login_as(users(:admin))
    visit root_path
  end

  def run_store_script(script)
    page.driver.with_playwright_page do |browser|
      browser.evaluate(<<~JS)
        async () => {
          const store = await import('controllers/offline_store');
          const tenant = `test:${crypto.randomUUID()}`;
          const take = await store.queueTake({ source_type: 'schedule', source_id: 1 }, tenant);
          const originalFetch = window.fetch;
          try { #{script} } finally { window.fetch = originalFetch; }
        }
      JS
    end
  end

  [408, 429, 500, 503].each do |status|
    it "retains and retries the original dose after HTTP #{status}" do
      result = run_store_script(<<~JS)
        window.fetch = async () => new Response('{}', { status: #{status}, headers: { 'Content-Type': 'application/json' } });
        const first = await store.syncQueuedTakes('/test-sync', tenant);
        const pending = await store.getQueuedTakes(tenant);
        window.fetch = async () => new Response(JSON.stringify({ data: { client_uuid: take.client_uuid } }), { headers: { 'Content-Type': 'application/json' } });
        await store.syncQueuedTakes('/test-sync', tenant);
        return { retryable: first.retryable, uuid: pending[0]?.client_uuid === take.client_uuid, remaining: (await store.getQueuedTakes(tenant)).length };
      JS

      expect(result).to eq('retryable' => true, 'uuid' => true, 'remaining' => 0)
    end
  end

  it 'retains doses when the network fails or a successful response is malformed' do
    result = run_store_script(<<~JS)
      window.fetch = async () => { throw new TypeError('Network unavailable'); };
      const network = await store.syncQueuedTakes('/test-sync', tenant);
      window.fetch = async () => new Response('invalid', { headers: { 'Content-Type': 'application/json' } });
      const malformed = await store.syncQueuedTakes('/test-sync', tenant);
      return { network: network.retryable, malformed: malformed.retryable, pending: (await store.getQueuedTakes(tenant)).length };
    JS

    expect(result).to eq('network' => true, 'malformed' => true, 'pending' => 1)
  end

  it 'retains authentication failures and does not touch another tenant' do
    result = run_store_script(<<~JS)
      await store.queueTake({ source_type: 'schedule', source_id: 2 }, tenant + ':other');
      window.fetch = async () => new Response('{}', { status: 401, headers: { 'Content-Type': 'application/json' } });
      const result = await store.syncQueuedTakes('/test-sync', tenant);
      return { auth: result.authRequired, pending: (await store.getQueuedTakes(tenant)).length, other: (await store.getQueuedTakes(tenant + ':other')).length };
    JS

    expect(result).to eq('auth' => true, 'pending' => 1, 'other' => 1)
  end

  it 'rolls back queue removal if storing a permanent rejection aborts' do
    result = run_store_script(<<~JS)
      const originalPut = IDBObjectStore.prototype.put;
      IDBObjectStore.prototype.put = function(...args) {
        const request = originalPut.apply(this, args);
        if (this.name === 'failedTakes') this.transaction.abort();
        return request;
      };
      window.fetch = async () => new Response(JSON.stringify({ error: { message: 'Unavailable' } }), { status: 422, headers: { 'Content-Type': 'application/json' } });
      try { await store.syncQueuedTakes('/test-sync', tenant); } catch (_) {}
      finally { IDBObjectStore.prototype.put = originalPut; }
      return { pending: (await store.getQueuedTakes(tenant)).length, failed: (await store.getFailedTakes(tenant)).length };
    JS

    expect(result).to eq('pending' => 1, 'failed' => 0)
  end

  it 'retains permanent rejections for review' do
    result = run_store_script(<<~JS)
      window.fetch = async () => new Response(JSON.stringify({ error: { message: 'Unavailable' } }), { status: 422, headers: { 'Content-Type': 'application/json' } });
      await store.syncQueuedTakes('/test-sync', tenant);
      return { pending: (await store.getQueuedTakes(tenant)).length, failed: (await store.getFailedTakes(tenant)).length };
    JS

    expect(result).to eq('pending' => 0, 'failed' => 1)
  end

  it 'closes database connections after reads' do
    result = run_store_script(<<~JS)
      await new Promise(resolve => setTimeout(resolve, 0));
      const originalClose = IDBDatabase.prototype.close;
      let closed = 0;
      IDBDatabase.prototype.close = function() { closed += 1; return originalClose.call(this); };
      try {
        await store.getValue('missing');
        await store.getQueuedTakes(tenant);
        await store.getFailedTakes(tenant);
        return closed;
      } finally { IDBDatabase.prototype.close = originalClose; }
    JS

    expect(result).to eq(3)
  end

  it 'shows permanent rejections without offering an ineffective retry' do
    visit offline_path
    expect(page).to have_css('[data-offline-shell-target="snapshotAge"]', text: /ago|Just now/)
    page.driver.with_playwright_page do |browser|
      browser.evaluate(<<~JS)
        async () => {
          const store = await import('controllers/offline_store');
          await store.saveFailedTake({ client_uuid: crypto.randomUUID() }, 'Dose rejected');
          window.dispatchEvent(new CustomEvent('medtracker:offline-take-queued'));
        }
      JS
    end

    expect(page).to have_text('Dose rejected')
    expect(page).to have_no_button('Retry sync')
  end

  it 'explains a temporary sync failure and exposes retry without losing the dose' do
    visit offline_path
    expect(page).to have_css('[data-offline-shell-target="snapshotAge"]', text: /ago|Just now/)
    page.driver.with_playwright_page do |browser|
      browser.evaluate(<<~JS)
        async () => {
          const store = await import('controllers/offline_store');
          const element = document.querySelector('[data-controller="offline-shell"]');
          const shell = window.Stimulus.getControllerForElementAndIdentifier(element, 'offline-shell');
          await store.queueTake({ source_type: 'schedule', source_id: 1 }, shell.tenantKeyValue);
          const originalFetch = window.fetch;
          window.fetch = async () => new Response('{}', { status: 503, headers: { 'Content-Type': 'application/json' } });
          try { await shell.retrySync(); } finally { window.fetch = originalFetch; }
        }
      JS
    end

    expect(page).to have_text('Your doses are saved on this device.')
    expect(page).to have_button('Retry sync')
    expect(page).to have_css('[data-offline-shell-target="pendingCount"]', text: '1')
  end
end
