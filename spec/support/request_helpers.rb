# frozen_string_literal: true

# Helper methods for request specs (non-Capybara HTTP tests)
module RequestHelpers
  TENANT_PATH_HELPERS = %i[
    add_medication_path ai_medication_suggestions_path dashboard_path
    admin_root_path admin_nhs_dmd_import_path new_admin_nhs_dmd_import_path
    admin_users_path admin_user_path new_admin_user_path edit_admin_user_path
    activate_admin_user_path verify_admin_user_path
    admin_invitations_path admin_invitation_path resend_admin_invitation_path
    admin_carer_relationships_path admin_carer_relationship_path new_admin_carer_relationship_path
    activate_admin_carer_relationship_path
    admin_people_path admin_audit_logs_path admin_audit_log_path admin_settings_path
    locations_path location_path new_location_path edit_location_path
    location_location_memberships_path location_location_membership_path
    medication_finder_path medication_finder_search_path
    medications_path medication_path new_medication_path edit_medication_path
    administration_medication_path nhs_guidance_medication_path refill_medication_path
    adjust_inventory_medication_path mark_as_ordered_medication_path mark_as_received_medication_path
    scan_restock_medications_path scan_restock_match_medications_path
    native_device_tokens_path native_device_token_path
    notification_preference_path offline_path offline_snapshot_path offline_medication_takes_path
    people_path person_path new_person_path edit_person_path add_medication_person_path
    person_avatar_path
    person_carer_relationships_path person_carer_relationship_path
    person_medication_assignments_path new_person_medication_assignment_path
    person_person_medications_path person_person_medication_path new_person_person_medication_path
    edit_person_person_medication_path reorder_person_person_medication_path
    take_medication_person_person_medication_path
    person_schedules_path person_schedule_path new_person_schedule_path edit_person_schedule_path
    take_medication_person_schedule_path
    profile_path profile_avatar_path push_subscription_path reports_path
    schedules_path schedule_path schedules_workflow_path start_schedules_workflow_path
    schedules_frequency_preview_path schedule_medication_takes_path
    search_path
  ].freeze

  TENANT_PATH_HELPERS.each do |helper_name|
    define_method(helper_name) do |*args, **options|
      if options.key?(:household_slug)
        super(*args, **options)
      else
        super(default_request_household_slug, *args, **options)
      end
    end
  end

  # Signs in a user via Rodauth for request specs.
  # Uses direct HTTP POST instead of Capybara page interactions.
  # Clears any 2FA setup to allow direct login without TOTP.
  def sign_in(user)
    account = Account.find_by(email: user.email_address)
    household = ensure_api_household_for(user)
    default_url_options[:household_slug] = household.slug

    # Clear 2FA to allow direct login
    clear_2fa_for_account(account)

    post '/login', params: { email: account.email, password: 'password' }
    follow_redirect! if response.redirect?
  end

  def default_request_household_slug
    default_url_options[:household_slug].presence || Household.first&.slug || create_default_request_household.slug
  end

  def create_default_request_household
    Household.create!(name: "Request Helper #{SecureRandom.hex(4)}", slug: "request-helper-#{SecureRandom.hex(4)}")
  end

  def household_dom_target(target)
    household = Household.find_by!(slug: default_request_household_slug)
    "household_#{household.id}_#{target}"
  end
end

RSpec.configure do |config|
  config.include RequestHelpers, type: :request
end
