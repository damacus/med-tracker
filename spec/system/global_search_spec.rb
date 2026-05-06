# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Global search command palette' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages,
           :schedules, :carer_relationships, :person_medications

  before do
    login_as(users(:jane))
  end

  scenario 'opens with Ctrl+K and Cmd+K, searches, navigates with arrow keys, and closes with Escape' do
    visit root_path

    page.execute_script('document.querySelector("a[href=\"/people\"]").focus()')
    expect(page.evaluate_script('document.activeElement.getAttribute("href")')).to eq('/people')

    first('a[href="/people"]').send_keys([:control, 'k'])

    expect(page).to have_css('dialog[open][aria-label="Global search"]')
    expect(page.evaluate_script('document.activeElement.id')).to eq('global_search_query')

    find_by_id('global_search_query').send_keys(:escape)
    expect(page).to have_no_css('dialog[open][aria-label="Global search"]')
    expect(page.evaluate_script('document.activeElement.getAttribute("href")')).to eq('/people')

    page.execute_script(
      'window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }))'
    )
    expect(page).to have_css('dialog[open][aria-label="Global search"]')

    find_by_id('global_search_query').send_keys(:escape)
    expect(page).to have_no_css('dialog[open][aria-label="Global search"]')
    expect(page.evaluate_script('document.activeElement.getAttribute("href")')).to eq('/people')

    first('a[href="/people"]').send_keys([:control, 'k'])
    expect(page).to have_css('dialog[open][aria-label="Global search"]')

    fill_in 'Search MedTracker', with: 'Vitamin D'

    expect(page).to have_link('Vitamin D')

    find_by_id('global_search_query').send_keys(:down)
    active_title = page.evaluate_script(
      'document.querySelector("[data-global-search-active=\"true\"]")?.textContent'
    )
    expect(active_title).to include('Vitamin D')

    find_by_id('global_search_query').send_keys(:enter)
    expect(page).to have_current_path(medication_path(medications(:vitamin_d)))

    expect(page).to have_css('button[aria-label="Open global search"]')
    find('button[aria-label="Open global search"]').send_keys([:control, 'k'])
    expect(page).to have_css('dialog[open][aria-label="Global search"]')

    find_by_id('global_search_query').send_keys(:escape)
    expect(page).to have_no_css('dialog[open][aria-label="Global search"]')
  end

  scenario 'opens from the mobile trigger' do
    page.current_window.resize_to(375, 667)
    visit root_path

    find('button[aria-label="Open global search"]').click

    expect(page).to have_css('dialog[open][aria-label="Global search"]')
    expect(page.evaluate_script('document.activeElement.id')).to eq('global_search_query')
  end
end
