# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Mobile overflow handling' do
  fixtures :all

  before do
    login_as(users(:admin))
  end

  it 'keeps medication header actions inside the mobile viewport', :js do
    page.current_window.resize_to(390, 844)

    visit medications_path

    expect(page_horizontal_overflow).to be <= 1
    expect(offscreen_header_actions).to be_empty
  end

  it 'keeps removed quick action chrome from causing mobile overflow', :js do
    page.current_window.resize_to(390, 844)

    visit dashboard_path

    expect(page).to have_css('[data-testid="mobile-rail"]')
    expect(page).to have_no_css('[data-testid="floating-action-menu-toggle"]')
    expect(page).to have_no_css('[data-testid="floating-action-menu-items"]')
    expect(page_horizontal_overflow).to be <= 1
  end

  it 'uses mobile cards without page-level overflow on dense table pages', :js do
    page.current_window.resize_to(390, 844)

    {
      schedules_path => 'schedules-mobile-list',
      admin_users_path => 'admin-users-mobile-list',
      admin_carer_relationships_path => 'admin-carer-relationships-mobile-list',
      admin_audit_logs_path => 'admin-audit-logs-mobile-list'
    }.each do |path, testid|
      visit path

      expect(page).to have_css(%([data-testid="#{testid}"]))
      expect(page).to have_no_table
      expect(page_horizontal_overflow).to be <= 1
    end
  end

  it 'keeps desktop tables visible on dense table pages', :js do
    page.current_window.resize_to(1024, 900)

    {
      schedules_path => 'schedules-desktop-table',
      admin_users_path => 'admin-users-desktop-table',
      admin_carer_relationships_path => 'admin-carer-relationships-desktop-table',
      admin_audit_logs_path => 'admin-audit-logs-desktop-table'
    }.each do |path, testid|
      visit path

      expect(page).to have_css(%([data-testid="#{testid}"] table), visible: :visible)
    end
  end

  def page_horizontal_overflow
    page.evaluate_script(<<~JS)
      (() => {
        const width = Math.max(document.documentElement.scrollWidth, document.body.scrollWidth);
        return width - document.documentElement.clientWidth;
      })()
    JS
  end

  def offscreen_header_actions
    page.evaluate_script(<<~JS)
      (() => {
        const viewportWidth = document.documentElement.clientWidth;
        return Array.from(document.querySelectorAll('.medications-index-actions a, .medications-index-actions button'))
          .filter((element) => {
            const styles = window.getComputedStyle(element);
            const rect = element.getBoundingClientRect();
            return styles.display !== 'none' && rect.width > 0 && (rect.left < -1 || rect.right > viewportWidth + 1);
          })
          .map((element) => element.textContent.trim());
      })()
    JS
  end
end
