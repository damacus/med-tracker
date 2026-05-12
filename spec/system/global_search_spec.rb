# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Global search command palette' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages,
           :schedules, :carer_relationships, :person_medications

  before do
    login_as(users(:jane))
  end

  scenario 'opens as a left-anchored dropdown with Ctrl+K and Cmd+K, searches, navigates, and closes' do
    visit root_path
    sleep 0.5

    page.execute_script('document.querySelector("a[href=\"/people\"]").focus()')
    expect(page.evaluate_script('document.activeElement.getAttribute("href")')).to eq('/people')

    first('a[href="/people"]').send_keys([:control, 'k'])

    expect(page).to have_css('#global_search_panel[aria-hidden="false"]')
    expect(page).to have_no_css('dialog[open][aria-label="Global search"]')
    expect(page.evaluate_script('document.activeElement.id')).to eq('global_search_query')
    expect(global_search_geometry['panel_left']).to be_within(1).of(global_search_geometry['trigger_left'])
    expect(global_search_geometry['panel_top']).to be > global_search_geometry['trigger_bottom']
    expect(global_search_geometry['panel_width']).to be >= global_search_geometry['trigger_width']

    find_by_id('global_search_query').send_keys(:escape)
    expect(page).to have_css('#global_search_panel[hidden]', visible: :all)
    expect(page.evaluate_script('document.activeElement.getAttribute("href")')).to eq('/people')

    first('a[href="/people"]').send_keys([:control, 'k'])
    find_by_id('global_search_query').send_keys(:escape)
    find('aside button[aria-label="Open global search"]').click
    expect(page).to have_css('#global_search_panel[aria-hidden="false"]')
    sleep 0.2
    expect(page).to have_css('#global_search_panel[aria-hidden="false"]')
    find_by_id('global_search_query').send_keys(:escape)
    page.execute_script('document.querySelector("a[href=\"/people\"]").focus()')

    page.execute_script(
      'window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }))'
    )
    expect(page).to have_css('#global_search_panel[aria-hidden="false"]')

    find_by_id('global_search_query').send_keys(:escape)
    expect(page).to have_css('#global_search_panel[hidden]', visible: :all)
    expect(page.evaluate_script('document.activeElement.getAttribute("href")')).to eq('/people')

    first('a[href="/people"]').send_keys([:control, 'k'])
    expect(page).to have_css('#global_search_panel[aria-hidden="false"]')

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
    page.execute_script(
      'window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", ctrlKey: true, bubbles: true }))'
    )
    expect(page).to have_css('#global_search_panel[aria-hidden="false"]')

    find_by_id('global_search_query').send_keys(:escape)
    expect(page).to have_css('#global_search_panel[hidden]', visible: :all)
  end

  scenario 'opens from the mobile trigger' do
    page.current_window.resize_to(375, 667)
    visit root_path
    sleep 0.5

    find('button[aria-label="Open global search"]').click

    expect(page).to have_css('#global_search_panel[aria-hidden="false"]')
    expect(page.evaluate_script('document.activeElement.id')).to eq('global_search_query')
  end

  def global_search_geometry
    page.evaluate_script(<<~JS)
      (() => {
        const panel = document.querySelector('#global_search_panel')
        const panelRect = panel.getBoundingClientRect()
        const triggerRect = document.querySelector('aside button[aria-label="Open global search"]').getBoundingClientRect()

        return {
          trigger_left: triggerRect.left,
          trigger_bottom: triggerRect.bottom,
          trigger_width: triggerRect.width,
          panel_left: panelRect.left,
          panel_top: panelRect.top,
          panel_width: panelRect.width
        }
      })()
    JS
  end
end
