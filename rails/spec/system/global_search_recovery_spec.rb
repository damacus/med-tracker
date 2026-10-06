# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Global search recovery', :browser do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications

  before do
    login_as(users(:jane))
    visit root_path
  end

  %w[http network malformed].each do |failure|
    it "shows an error and retries the retained query after a #{failure} failure" do
      expect(page).to have_css('body[data-global-search-connected="true"]', visible: :all)
      page.execute_script(<<~JS)
        window.originalSearchFetch = window.fetch;
        window.fetch = function(input, init) {
          if (!String(input).includes('/search.json')) return window.originalSearchFetch(input, init);
          if ('#{failure}' === 'network') return Promise.reject(new TypeError('Network unavailable'));
          return Promise.resolve(new Response('#{failure == 'malformed' ? '{}' : 'Unavailable'}', {
            status: #{failure == 'http' ? 503 : 200}, headers: { 'Content-Type': 'application/json' }
          }));
        };
      JS

      find('aside button[aria-label="Open global search"]').click
      fill_in 'Search MedTracker', with: 'Vitamin D'
      expect(page).to have_text('Search is unavailable. Please try again.')
      expect(page).to have_no_text('No results')

      page.execute_script('window.fetch = window.originalSearchFetch')
      click_button 'Retry search'
      expect(page).to have_link('Vitamin D')
      expect(page).to have_field('Search MedTracker', with: 'Vitamin D')
      expect(page).to have_no_text('Search is unavailable. Please try again.')
    end
  end
end
