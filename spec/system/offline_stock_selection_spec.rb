# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Offline stock selection', :browser do
  fixtures :accounts, :people, :users

  it 'uses remaining stock and queues only once during overlapping clicks' do
    login_as(users(:admin))
    visit offline_path
    expect(page).to have_css('[data-offline-shell-target="snapshotAge"]', text: /ago|Just now/)

    result = page.driver.with_playwright_page do |browser|
      browser.evaluate(<<~JS)
        async () => {
          const store = await import('controllers/offline_store');
          const { default: Shell } = await import('controllers/offline_shell_controller');
          const tenant = `test:${crypto.randomUUID()}`;
          const medication = { id: 501, name: 'Test medicine', dose_amount: 1, dose_unit: 'tablet', current_supply: 1 };
          const source = { id: 901, person_id: 801, medication_id: 501, dose_amount: 1, dose_unit: 'tablet' };
          const payload = { data: { medications: [medication, { ...medication, id: 502 }], schedules: [source] } };
          await store.saveSnapshot(payload, tenant);
          await store.queueTake({ source_type: 'schedule', source_id: 900, taken_from_medication_id: 501, dose_amount: 1, dose_unit: 'tablet' }, tenant);
          const shell = Object.create(Shell.prototype);
          Object.defineProperty(shell, 'tenantKeyValue', { value: tenant });
          const button = document.createElement('button');
          Object.assign(button.dataset, { sourceType: 'schedule', sourceId: '901', doseAmount: '1', doseUnit: 'tablet' });
          await Promise.all([shell.queue({ currentTarget: button }), shell.queue({ currentTarget: button })]);
          return (await store.getQueuedTakes(tenant)).filter(take => take.source_id === 901).map(take => take.taken_from_medication_id);
        }
      JS
    end

    expect(result).to eq([502])
  end
end
