# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Global search command palette', :browser do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages,
           :schedules, :carer_relationships, :person_medications

  before do
    login_as(users(:jane))
  end

  scenario 'opens as a left-anchored dropdown with Ctrl+K and Cmd+K, searches, navigates, and closes' do
    visit root_path
    expect(page).to have_css('body[data-global-search-connected="true"]', visible: :all)

    page.execute_script("document.querySelector('a[href=\"#{people_path}\"]').focus()")
    expect(page.evaluate_script('document.activeElement.getAttribute("href")')).to eq(people_path)

    open_global_search_shortcut

    expect(page).to have_css('#global_search_panel[aria-hidden="false"]')
    expect(page).to have_no_css('dialog[open][aria-label="Global search"]')
    expect(page.evaluate_script('document.activeElement.id')).to eq('global_search_query')
    expect(global_search_geometry['panel_left']).to be_within(1).of(global_search_geometry['trigger_left'])
    expect(global_search_geometry['panel_top']).to be > global_search_geometry['trigger_bottom']
    expect(global_search_geometry['panel_width']).to be >= global_search_geometry['trigger_width']

    find_by_id('global_search_query').send_keys(:escape)
    expect(page).to have_css('#global_search_panel[hidden]', visible: :all)
    expect(page.evaluate_script('document.activeElement.getAttribute("href")')).to eq(people_path)

    open_global_search_shortcut
    find_by_id('global_search_query').send_keys(:escape)
    find('aside button[aria-label="Open global search"]').click
    expect(page).to have_css('#global_search_panel[aria-hidden="false"]')
    find_by_id('global_search_query').send_keys(:escape)
    page.execute_script("document.querySelector('a[href=\"#{people_path}\"]').focus()")

    open_global_search_shortcut(modifier: :meta)
    expect(page).to have_css('#global_search_panel[aria-hidden="false"]')

    find_by_id('global_search_query').send_keys(:escape)
    expect(page).to have_css('#global_search_panel[hidden]', visible: :all)
    expect(page.evaluate_script('document.activeElement.getAttribute("href")')).to eq(people_path)

    open_global_search_shortcut
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

    expect(page).to have_css('body[data-global-search-connected="true"]', visible: :all)
    expect(page).to have_css('button[aria-label="Open global search"]')
    open_global_search_shortcut
    expect(page).to have_css('#global_search_panel[aria-hidden="false"]')

    find_by_id('global_search_query').send_keys(:escape)
    expect(page).to have_css('#global_search_panel[hidden]', visible: :all)
  end

  scenario 'opens from the mobile trigger' do
    page.current_window.resize_to(375, 667)
    visit root_path
    expect(page).to have_css('body[data-global-search-connected="true"]', visible: :all)

    find('button[aria-label="Open global search"]').click

    expect(page).to have_css('#global_search_panel[aria-hidden="false"]')
    expect(page.evaluate_script('document.activeElement.id')).to eq('global_search_query')
  end

  scenario 'does not request empty search results when opened' do
    visit root_path
    page.execute_script(<<~JS)
      window.__searchFetches = [];
      window.__originalFetch = window.fetch;
      window.fetch = function(input, init) {
        const url = typeof input === "string" ? input : input.url;
        if (url.includes("/search.json")) window.__searchFetches.push(url);
        return window.__originalFetch.call(window, input, init);
      };
    JS

    find('button[aria-label="Open global search"]').click

    expect(page).to have_css('#global_search_panel[aria-hidden="false"]')
    expect(page.evaluate_script('window.__searchFetches')).to be_empty
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

  def open_global_search_shortcut(modifier: :ctrl)
    modifier_key = modifier == :meta ? 'metaKey' : 'ctrlKey'

    page.execute_script(<<~JS)
      document.activeElement.dispatchEvent(
        new KeyboardEvent("keydown", { key: "k", #{modifier_key}: true, bubbles: true })
      )
    JS
  end
end
