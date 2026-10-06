# frozen_string_literal: true

# This module contains helper methods for system tests, using the Capybara DSL.
module SystemHelpers
  TENANT_PATH_HELPERS = RequestHelpers::TENANT_PATH_HELPERS

  TENANT_PATH_HELPERS.each do |helper_name|
    define_method(helper_name) do |*args, **options|
      if options.key?(:household_slug)
        super(*args, **options)
      else
        super(default_browser_household_slug, *args, **options)
      end
    end
  end

  # Signs in a user using the login form.
  # This helper uses standard Capybara methods to interact with the page.
  # Clears any 2FA setup to allow direct login without TOTP.
  def sign_in(user)
    account = prepare_browser_session_for(user)

    visit '/login'
    fill_in 'Email address', with: user.email_address
    fill_in 'Password', with: 'password'
    click_button 'Sign In to Dashboard'

    expect(page).to have_current_path(expected_dashboard_path_for(account.email))
  end

  def prepare_browser_session_for(user)
    household_user = household_user_for(user)
    household = ensure_api_household_for(household_user) if household_user
    @browser_household = household if household
    @browser_membership = browser_membership_for(household_user, household) if household_user && household

    account = account_for_authentication(user)
    clear_2fa_for_account(account) if account.respond_to?(:id)
    account
  end

  def default_browser_household_slug
    @browser_household&.slug || Current.household&.slug || first_browser_household_slug
  end

  def first_browser_household_slug
    Household.first&.slug || create_default_browser_household.slug
  end

  def create_default_browser_household
    Household.create!(name: "System Helper #{SecureRandom.hex(4)}", slug: "system-helper-#{SecureRandom.hex(4)}")
  end

  def browser_household
    @browser_household || Household.find_by(slug: default_browser_household_slug)
  end

  def browser_membership
    @browser_membership
  end

  def grant_browser_access(person, access_level: :manage)
    grant = PersonAccessGrant.find_or_initialize_by(
      household: browser_household,
      household_membership: browser_membership,
      person: person
    )
    grant.access_level = access_level
    grant.relationship_type = :family_member
    grant.granted_by_membership = browser_membership
    grant.save!
    grant
  end

  def tenant_dom_id(record, prefix = nil)
    target = ActionView::RecordIdentifier.dom_id(record, prefix)
    household = browser_household

    household ? "household_#{household.id}_#{target}" : target
  end
end

RSpec.configure do |config|
  # Include these helpers in all system tests.
  config.include SystemHelpers, type: :system
  config.include SystemHelpers, type: :feature
end
