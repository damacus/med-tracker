# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Mobile overflow handling' do
  fixtures :all

  before do
    admin = users(:admin)
    login_as(admin)
    create_household_audit_version(admin)
  end

  after do
    PaperTrail.request.controller_info = {}
    PaperTrail.request.whodunnit = nil
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

  it 'keeps overflow diagnostics privacy-safe', :js do
    visit root_path
    page.execute_script(<<~JS)
      const probe = document.createElement('div');
      probe.textContent = 'fixture name and medication details';
      probe.style.cssText = 'position:absolute; left:100vw; width:40px; height:24px;';
      document.body.appendChild(probe);
    JS

    diagnostics = page.evaluate_script(Rails.root.join('spec/support/mobile_ui_overflowing_elements.js').read)

    expect(diagnostics).to all(
      include('tag', 'id', 'className', 'left', 'right', 'width')
    )
    expect(diagnostics).to all(satisfy { |entry| !entry.key?('text') })
    expect(diagnostics.join).not_to include('fixture name')
    expect(diagnostics.join).not_to include('medication details')
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
    page.evaluate_script(offscreen_header_actions_script)
  end

  def offscreen_header_actions_script
    <<~JS
      (() => {
        const viewportWidth = document.documentElement.clientWidth;
        return Array.from(document.querySelectorAll('.medications-index-actions a, .medications-index-actions button'))
          .filter((element) => {
            const styles = window.getComputedStyle(element);
            const rect = element.getBoundingClientRect();
            return styles.display !== 'none' && rect.width > 0 && (rect.left < -1 || rect.right > viewportWidth + 1);
          })
          .map((element) => {
            const rect = element.getBoundingClientRect();
            return {
              selector: element.id ? `#${element.id}` : element.tagName.toLowerCase(),
              role: element.getAttribute('role'),
              left: Math.round(rect.left),
              right: Math.round(rect.right),
              width: Math.round(rect.width),
              height: Math.round(rect.height),
              viewport: { width: window.innerWidth, height: window.innerHeight }
            };
          });
      })()
    JS
  end

  def create_household_audit_version(admin)
    PaperTrail::Version.create!(
      household_id: browser_household.id,
      actor_membership_id: browser_membership&.id,
      item_type: 'User',
      item_id: admin.id,
      event: 'update',
      whodunnit: admin.id.to_s,
      created_at: Time.current
    )
  end
end
