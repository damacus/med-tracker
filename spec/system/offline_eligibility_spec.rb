# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Offline dose eligibility', :browser do
  fixtures :accounts, :people, :users

  before do
    login_as(users(:admin))
    visit offline_path
  end

  it 'disables restricted, stale and overlapping pending doses with a reason' do
    expect(page).to have_css('[data-offline-shell-target="snapshotAge"]', text: /ago|Just now/)
    result = page.driver.with_playwright_page do |browser|
      browser.evaluate(<<~JS)
        async () => {
          const element = document.querySelector('[data-controller="offline-shell"]');
          const shell = window.Stimulus.getControllerForElementAndIdentifier(element, 'offline-shell');
          const medication = { id: 1, name: 'Test medicine', dose_amount: 1, dose_unit: 'tablet', current_supply: 10 };
          const base = { id: 1, person_id: 1, medication_id: 1, dose_amount: 1, dose_unit: 'tablet' };
          const eligibility = { allowed: true, valid_until: new Date(Date.now() + 60000).toISOString(), dose_amount: '1', dose_unit: 'tablet' };
          const scenarios = [
            [{ ...base, offline_eligibility: { ...eligibility, allowed: false, reason: 'Paused' } }, []],
            [{ ...base, offline_eligibility: { ...eligibility, valid_until: '2000-01-01T00:00:00Z' } }, []],
            [base, []],
            [{ ...base, offline_eligibility: eligibility }, [{ source_type: 'schedule', source_id: 1 }]]
          ];
          return scenarios.map(([source, queued]) => {
            shell.renderToday({ medications: [medication], people: [{ id: 1, name: 'Person' }], schedules: [source] }, queued);
            return { disabled: shell.todayTarget.querySelector('button').disabled, reason: shell.todayTarget.textContent.includes('Paused') || shell.todayTarget.textContent.includes('Refresh') || shell.todayTarget.textContent.includes('Sync') };
          });
        }
      JS
    end

    expect(result).to all(eq('disabled' => true, 'reason' => true))
  end
end
