# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Mobile UI audit' do
  fixtures :all

  before do
    login_as(users(:admin))
    page.current_window.resize_to(390, 844)
    create_audit_log_entry
  end

  after do
    PaperTrail.request.controller_info = {}
    PaperTrail.request.whodunnit = nil
  end

  it 'keeps stock inventory meters aligned in light and dark mode', :js do
    %w[light dark].each do |appearance|
      apply_appearance(appearance)
      visit dashboard_path(dashboard_person_id: DashboardPresenter::ALL_FAMILY_PERSON_ID)

      expect(page).to have_css('[data-testid="dashboard-stock-meter"]')
      expect(stock_meter_geometry).to all(
        include('contained' => true, 'centered' => true, 'usesIndicatorColor' => true, 'avoidsFallbackBlack' => true)
      )
      expect(page_horizontal_overflow).to be <= 1
    end
  end

  it 'keeps authenticated UI routes readable without mobile overflow in dark mode', :js do
    apply_appearance('dark')

    audited_ui_paths.each do |path|
      visit path

      expect(page).to have_css('body')
      expect(page_horizontal_overflow).to be <= 1,
                                          format(
                                            'overflow=%<overflow>s diagnostics=%<diagnostics>s',
                                            overflow: page_horizontal_overflow,
                                            diagnostics: overflowing_elements.inspect
                                          )
      expect(low_contrast_text).to be_empty, "contrast_failures=#{low_contrast_text.inspect}"
    end
  end

  it 'keeps signed-out UI routes readable without mobile overflow in dark mode', :js do
    invitation = create(:household_invitation, email: 'mobile-audit@example.org', membership_role: :member)

    rodauth_logout
    apply_appearance('dark')

    [
      login_path,
      create_account_path,
      accept_invitation_path(token: invitation.token)
    ].each do |path|
      visit path

      expect(page_horizontal_overflow).to be <= 1, "overflow=#{page_horizontal_overflow}"
      expect(low_contrast_text).to be_empty, "contrast_failures=#{low_contrast_text.inspect}"
    end
  end

  it 'audits the signed-out accessibility journey at desktop and mobile widths', :js do
    rodauth_logout
    invitation = create(:household_invitation, email: 'accessibility-harness@example.org', membership_role: :member)
    create_account = "#{create_account_path}?invitation_token=#{CGI.escape(invitation.token)}"

    [login_path, create_account].each do |path|
      [1280, 390].each do |width|
        page.current_window.resize_to(width, width == 390 ? 844 : 800)
        visit path
        assert_accessibility_contract
      end
    end
  end

  it 'audits the administrator journey at desktop and mobile widths', :js do
    [1280, 390].each do |width|
      page.current_window.resize_to(width, width == 390 ? 844 : 800)

      visit admin_root_path
      assert_accessibility_contract
      assert_visible_navigation(admin_root_path, admin_users_path, medications_path)

      visit new_medication_path
      assert_accessibility_contract

      visit admin_users_path
      assert_accessibility_contract
      if width == 390
        expect(page).to have_css('[data-testid="admin-users-mobile-list"]')
        expect(page).to have_no_table
      else
        expect(page).to have_css('[data-testid="admin-users-desktop-table"] table thead th')
      end
    end
  end

  it 'audits the carer journey without unrelated data or mutation controls', :js do
    login_as(users(:carer))
    assigned_person = people(:child_patient)
    unrelated_person = people(:john)

    [1280, 390].each do |width|
      page.current_window.resize_to(width, width == 390 ? 844 : 800)

      visit dashboard_path
      assert_accessibility_contract
      if width == 1280
        assert_visible_navigation(dashboard_path(dashboard_person_id: assigned_person.id))
      else
        expect(page).to have_css('[data-testid="dashboard-person-mobile-current"]')
      end
      assert_no_visible_navigation(dashboard_path(dashboard_person_id: unrelated_person.id))
      assert_no_visible_navigation('/admin')

      visit people_path
      assert_accessibility_contract
      expect(page).to have_css("##{tenant_dom_id(assigned_person)}")
      expect(page).to have_no_css("##{tenant_dom_id(unrelated_person)}")
      assert_no_visible_controls('New Person', 'Add Medication', 'Edit', 'Delete')

      visit person_path(assigned_person)
      assert_accessibility_contract
      expect(page).to have_css("#household_#{browser_household.id}_person_show_#{assigned_person.id}")
      assert_no_visible_navigation('/people/new', '/edit', '/delete')
      assert_no_visible_controls('New Person', 'Add Medication', 'Edit', 'Delete')

      next unless width == 390

      find('button[aria-label="Open menu"]').click
      expect(page).to have_css('[role="dialog"]:focus', wait: 2)
      assert_dialog_contract
      page.execute_script(%q{document.querySelector('[data-testid="drawer-backdrop"]').click()})
    end
  end

  def audited_ui_paths
    base_ui_paths + medication_ui_paths + schedule_ui_paths + people_ui_paths + admin_ui_paths
  end

  def base_ui_paths
    [root_path] + household_paths(
      :dashboard_path,
      :profile_path,
      :reports_path,
      :offline_path,
      :locations_path,
      :new_location_path
    ) + [
      household_path(:location_path, id: locations(:home)),
      household_path(:edit_location_path, id: locations(:home))
    ]
  end

  def medication_ui_paths
    household_paths(:medications_path, :new_medication_path, :medication_finder_path) + [
      household_path(:medication_path, id: medications(:paracetamol)),
      household_path(:edit_medication_path, id: medications(:paracetamol)),
      household_path(:administration_medication_path, id: medications(:paracetamol)),
      household_path(:nhs_guidance_medication_path, id: medications(:paracetamol))
    ]
  end

  def schedule_ui_paths
    household_paths(:schedules_path, :new_schedule_path, :schedules_workflow_path) + [
      household_path(:schedule_path, id: schedules(:john_paracetamol)),
      household_path(:edit_schedule_path, id: schedules(:john_paracetamol))
    ]
  end

  def people_ui_paths
    household_paths(:people_path, :new_person_path) + person_ui_paths + nested_people_ui_paths
  end

  def person_ui_paths
    [
      household_path(:person_path, id: people(:john)),
      household_path(:edit_person_path, id: people(:john)),
      household_path(:add_medication_person_path, id: people(:john))
    ]
  end

  def nested_people_ui_paths
    [
      household_path(:new_person_schedule_path, person_id: people(:john)),
      household_path(:edit_person_schedule_path, person_id: people(:john), id: schedules(:john_paracetamol)),
      household_path(:new_person_person_medication_path, person_id: people(:john)),
      household_path(
        :edit_person_person_medication_path,
        person_id: people(:john),
        id: person_medications(:john_vitamin_d)
      ),
      household_path(:new_person_carer_relationship_path, person_id: people(:john)),
      household_path(:new_person_medication_assignment_path, person_id: people(:john))
    ]
  end

  def admin_ui_paths
    household_paths(
      :admin_root_path,
      :new_admin_nhs_dmd_import_path,
      :admin_users_path,
      :new_admin_user_path,
      :admin_invitations_path,
      :admin_carer_relationships_path,
      :new_admin_carer_relationship_path,
      :admin_people_path,
      :admin_audit_logs_path,
      :admin_settings_path
    ) + [
      household_path(:edit_admin_user_path, id: users(:jane)),
      household_path(:admin_audit_log_path, id: PaperTrail::Version.last)
    ]
  end

  def household_paths(*helpers)
    helpers.map { |helper| household_path(helper) }
  end

  def household_path(helper, **params)
    public_send(helper, **household_route_params, **params)
  end

  def household_route_params
    { household_slug: browser_household.slug }
  end

  def apply_appearance(appearance)
    visit root_path
    page.execute_script(<<~JS)
      localStorage.setItem("med-tracker-appearance", "#{appearance}");
      document.documentElement.classList.toggle("dark", "#{appearance}" === "dark");
      document.documentElement.dataset.appearance = "#{appearance}";
    JS
  end

  def stock_meter_geometry
    page.evaluate_script(stock_meter_geometry_script)
  end

  def stock_meter_geometry_script
    <<~JS
      Array.from(document.querySelectorAll('[data-testid="dashboard-stock-meter"]')).map((meter) => {
        const fill = meter.querySelector('[data-testid="dashboard-stock-meter-fill"]');
        const meterRect = meter.getBoundingClientRect();
        const fillRect = fill.getBoundingClientRect();
        const meterCenter = meterRect.top + (meterRect.height / 2);
        const fillCenter = fillRect.top + (fillRect.height / 2);
        const fillStyle = getComputedStyle(fill);

        return {
          contained: fillRect.top >= meterRect.top - 0.5 && fillRect.bottom <= meterRect.bottom + 0.5,
          centered: Math.abs(meterCenter - fillCenter) <= 0.5,
          usesIndicatorColor: fillStyle.backgroundColor !== "rgba(0, 0, 0, 0)",
          avoidsFallbackBlack: fillStyle.backgroundColor !== "rgb(0, 0, 0)"
        };
      });
    JS
  end

  def page_horizontal_overflow
    page.evaluate_script(<<~JS)
      (() => {
        const width = Math.max(document.documentElement.scrollWidth, document.body.scrollWidth);
        return width - document.documentElement.clientWidth;
      })()
    JS
  end

  def overflowing_elements
    page.evaluate_script(Rails.root.join('spec/support/mobile_ui_overflowing_elements.js').read)
  end

  def low_contrast_text
    page.evaluate_script(low_contrast_text_script)
  end

  def low_contrast_text_script
    Rails.root.join('spec/support/mobile_ui_low_contrast_text.js').read
  end

  def assert_accessibility_contract
    contract = page.evaluate_script(Rails.root.join('spec/support/mobile_ui_accessibility.js').read)

    assert_contract_overflow(contract)
    assert_heading_order
    assert_actionable_names(contract)
    assert_actionable_targets(contract)
    assert_actionable_focus(contract)
    expect(low_contrast_text).to be_empty,
                                 "contrast_failures=#{low_contrast_text.inspect}"
  end

  def assert_contract_overflow(contract)
    expect(contract.fetch('overflow')).to be <= 1,
                                          format(
                                            'overflow=%<overflow>s viewport=%<viewport>s',
                                            overflow: contract.fetch('overflow'),
                                            viewport: contract.fetch('viewport').inspect
                                          )
  end

  def assert_heading_order
    failures = heading_order_failures
    expect(failures).to be_empty, "heading_failures=#{failures.inspect}"
  end

  def assert_actionable_names(contract)
    failures = contract.dig('actionable', 'missingNames')
    expect(failures).to be_empty, "name_failures=#{failures.inspect}"
  end

  def assert_actionable_targets(contract)
    failures = contract.dig('actionable', 'targetFailures')
    expect(failures).to be_empty, "target_failures=#{failures.inspect}"
  end

  def assert_actionable_focus(contract)
    failures = contract.dig('actionable', 'focusFailures')
    expect(failures).to be_empty, "focus_failures=#{failures.inspect}"
  end

  def assert_visible_navigation(*paths)
    paths_json = paths.map(&:to_s).to_json
    missing = page.evaluate_script(<<~JS)
      (() => {
        const paths = #{paths_json};
        const links = Array.from(document.querySelectorAll('a[href]'));
        const visible = (element) => {
          const styles = getComputedStyle(element);
          const rect = element.getBoundingClientRect();
          return styles.display !== 'none' && styles.visibility !== 'hidden' && rect.width > 0 && rect.height > 0;
        };
        return paths.filter((path) => !links.some((link) => visible(link) && link.getAttribute('href').includes(path)))
          .map(() => ({ selector: 'a[href]', role: 'link' }));
      })()
    JS

    expect(missing).to be_empty, "navigation_failures=#{missing.inspect}"
  end

  def heading_order_failures
    levels = page.all('main h1, main h2, main h3, main h4, main h5, main h6', visible: true)
                 .map { |heading| heading.tag_name.delete('h').to_i }
    failures = []
    if levels.any? && levels.first != 1
      failures << {
        selector: 'h1', role: 'heading', level: levels.first, expected: 1
      }
    end
    levels.each_cons(2) do |previous, level|
      failures << { selector: "h#{level}", role: 'heading', level:, previous: } if level > previous + 1
    end
    failures
  end

  def assert_no_visible_navigation(*paths)
    paths_json = paths.map(&:to_s).to_json
    unexpected = page.evaluate_script(<<~JS)
      (() => {
        const paths = #{paths_json};
        const links = Array.from(document.querySelectorAll('a[href]'));
        const visible = (element) => {
          const styles = getComputedStyle(element);
          const rect = element.getBoundingClientRect();
          return styles.display !== 'none' && styles.visibility !== 'hidden' && rect.width > 0 && rect.height > 0;
        };
        return links.filter((link) => visible(link) && paths.some((path) => link.getAttribute('href').includes(path)))
          .map((link) => ({ selector: link.id ? `#${link.id}` : 'a[href]', role: 'link' }));
      })()
    JS

    expect(unexpected).to be_empty, "unexpected_navigation=#{unexpected.inspect}"
  end

  def assert_no_visible_controls(*labels)
    labels_json = labels.to_json
    unexpected = page.evaluate_script(visible_controls_script(labels_json))

    expect(unexpected).to be_empty, "unexpected_controls=#{unexpected.inspect}"
  end

  def visible_controls_script(labels_json)
    <<~JS
      (() => {
        const labels = #{labels_json};
        const controls = Array.from(document.querySelectorAll('a, button, input, select, textarea, [role="button"]'));
        const visible = (element) => {
          const styles = getComputedStyle(element);
          const rect = element.getBoundingClientRect();
          return styles.display !== 'none' && styles.visibility !== 'hidden' && rect.width > 0 && rect.height > 0;
        };
        const name = (element) => element.getAttribute('aria-label') || element.textContent.trim();
        return controls.filter((control) => visible(control) && labels.includes(name(control)))
          .map((control) => ({ selector: control.id ? `#${control.id}` : control.tagName.toLowerCase(), role: control.getAttribute('role') || control.tagName.toLowerCase() }));
      })()
    JS
  end

  def assert_dialog_contract
    dialog = page.evaluate_script(dialog_contract_script)

    expect(dialog).to include(
      'present' => 1,
      'labelled' => 1,
      'modal' => 1,
      'focusInside' => 1
    ), "dialog=#{dialog.inspect}"
  end

  def dialog_contract_script
    <<~JS
      (() => {
        const element = Array.from(document.querySelectorAll('[role="dialog"]')).find((candidate) => {
          const styles = getComputedStyle(candidate);
          const rect = candidate.getBoundingClientRect();
          return styles.display !== 'none' && styles.visibility !== 'hidden' && rect.width > 0 && rect.height > 0;
        });
        const focusInside = element ? element.contains(document.activeElement) : false;
        return {
          present: element ? 1 : 0,
          labelled: element && (element.getAttribute('aria-label') || element.getAttribute('aria-labelledby')) ? 1 : 0,
          modal: element?.getAttribute('aria-modal') === 'true' ? 1 : 0,
          focusInside: focusInside ? 1 : 0,
          viewport: { width: window.innerWidth, height: window.innerHeight }
        };
      })()
    JS
  end

  def create_audit_log_entry
    PaperTrail::Version.create!(
      household_id: browser_household.id,
      actor_membership_id: browser_membership&.id,
      item_type: 'User',
      item_id: users(:admin).id,
      event: 'update',
      whodunnit: users(:admin).id.to_s,
      created_at: Time.current
    )
  end
end
