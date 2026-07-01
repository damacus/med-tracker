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
      expect(page_horizontal_overflow).to be <= 1, "Horizontal overflow on #{path}"
      expect(low_contrast_text).to be_empty, "Low contrast text on #{path}: #{low_contrast_text.inspect}"
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

      expect(page_horizontal_overflow).to be <= 1, "Horizontal overflow on #{path}"
      expect(low_contrast_text).to be_empty, "Low contrast text on #{path}: #{low_contrast_text.inspect}"
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
    public_send(helper, household_route_params.merge(params))
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

  def low_contrast_text
    page.evaluate_script(low_contrast_text_script)
  end

  def low_contrast_text_script
    Rails.root.join('spec/support/mobile_ui_low_contrast_text.js').read
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
