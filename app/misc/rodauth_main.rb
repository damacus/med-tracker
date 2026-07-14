# frozen_string_literal: true

require 'sequel/core'

# rubocop:disable Metrics/ClassLength
class RodauthMain < Rodauth::Rails::Auth
  # rubocop:disable Metrics/BlockLength -- Rodauth configuration DSL requires a single configure block
  configure do
    # List of authentication features that are loaded.
    enable :create_account, :verify_account, :verify_account_grace_period,
           :login, :logout, :remember, :lockout, :active_sessions,
           :reset_password, :change_password, :change_login, :verify_login_change,
           :close_account, :omniauth,
           :otp, :recovery_codes,
           :webauthn, :webauthn_login, :webauthn_autofill,
           :oauth_pkce, :oauth_token_revocation

    oauth_application_scopes OauthApplication::SUPPORTED_SCOPES
    oauth_access_token_expires_in 15.minutes.to_i
    oauth_refresh_token_expires_in 30.days.to_i
    oauth_refresh_token_protection_policy 'rotation'
    oauth_grants_token_hash_column :token_hash
    oauth_grants_refresh_token_hash_column :refresh_token_hash
    oauth_applications_client_secret_hash_column :client_secret_hash
    oauth_token_endpoint_auth_methods_supported %w[none client_secret_basic client_secret_post]

    # See the Rodauth documentation for the list of available config options:
    # http://rodauth.jeremyevans.net/documentation.html

    # Initialize Sequel and have it reuse Active Record's database connection.
    # Use appropriate adapter based on Rails database configuration
    db_adapter = ActiveRecord::Base.connection.adapter_name.downcase
    if db_adapter == 'postgresql'
      db Sequel.postgres(extensions: :activerecord_connection, keep_reference: false)
    else
      db Sequel.sqlite(extensions: :activerecord_connection, keep_reference: false)
    end
    # Avoid DB query that checks accounts table schema at boot time.
    convert_token_id_to_integer? { Account.columns_hash['id'].type == :integer }

    # Specify the controller used for view rendering, CSRF, and callbacks.
    rails_controller { RodauthController }

    # Make built-in page titles accessible in your views via an instance variable.
    title_instance_variable :@page_title

    # Store account status in an integer column without foreign key constraint.
    account_status_column :status

    # Store password hash in a column instead of a separate table.
    account_password_hash_column :password_hash
    verify_account_set_password? false

    # Autologin after account creation
    create_account_autologin? true

    # Set timestamps when creating accounts (Sequel doesn't do this automatically)
    new_account do |login|
      {
        email: login,
        status: account_initial_status_value,
        created_at: Time.current,
        updated_at: Time.current
      }
    end

    # Change some default param keys.
    login_param 'email'
    login_confirm_param 'email-confirm'
    send_email do |email|
      # queue email delivery on the mailer after the transaction commits
      db.after_commit { email.deliver_later }
    end

    email_from do
      Rails.application.credentials.dig(:mailer, :from) ||
        ENV.fetch('MAILER_FROM', 'MedTracker <noreply@medtracker.app>')
    end
    verify_account_email_subject { I18n.t('rodauth.verify_account.subject') }

    create_verify_account_email do
      audit_auth_token('verification_key', 'created')
      RodauthMailer.verify_account(self.class.configuration_name, account_id, verify_account_key_value)
    end

    reset_password_email_subject { I18n.t('rodauth.reset_password.subject') }
    create_reset_password_email do
      audit_auth_token('password_reset_key', 'created')
      RodauthMailer.reset_password(self.class.configuration_name, account_id, reset_password_key_value)
    end

    verify_login_change_email_subject { I18n.t('rodauth.verify_login_change.subject') }
    create_verify_login_change_email do |_login|
      audit_auth_token('login_change_key', 'created')
      RodauthMailer.verify_login_change(self.class.configuration_name, account_id, verify_login_change_key_value)
    end

    unlock_account_email_subject { I18n.t('rodauth.unlock_account.subject') }
    create_unlock_account_email do
      RodauthMailer.unlock_account(self.class.configuration_name, account_id, account_lockouts_key_value)
    end

    password_minimum_length 12
    # bcrypt has a maximum input length of 72 bytes, truncating any extra bytes.
    password_maximum_bytes 72 # add custom password complexity rules
    # Custom password complexity requirements (alternative to password_complexity feature).
    password_meets_requirements? do |password|
      super(password) && password_complex_enough?(password)
    end
    auth_class_eval do
      public :password_hash

      def resource_owner_params
        membership = Account.find(account_id).first_active_household_membership
        super.merge(
          household_membership_id: membership.id,
          person_id: membership.person_id,
          permissions_version: membership.permissions_version,
          created_at: Time.current,
          updated_at: Time.current
        )
      end

      def authorize_page_lead(name:)
        membership = Account.find(account_id).first_active_household_membership
        household_name = ERB::Util.html_escape(membership.household.name)
        person_name = ERB::Util.html_escape(membership.person.name)
        "#{super} for #{household_name} / #{person_name}"
      end

      def create_oauth_grant(create_params = {})
        code = super
        record_smart_oauth_event('smart_oauth.consent_granted', create_params.merge(resource_owner_params))
        code
      end

      def revoke_oauth_grant
        grant = super
        record_smart_oauth_event('smart_oauth.token_revoked', grant)
        grant
      end

      def record_smart_oauth_event(event_type, grant)
        membership = HouseholdMembership.find(grant.fetch(:household_membership_id))
        Audit::Event.record!(
          household_id: membership.household_id,
          actor_account_id: membership.account_id,
          actor_membership_id: membership.id,
          event_type: event_type,
          request_id: rails_controller_instance&.request&.request_id,
          ip: request.ip,
          metadata: smart_oauth_audit_metadata(grant, membership)
        )
      end

      def smart_oauth_audit_metadata(grant, membership)
        {
          oauth_application_id: oauth_application.fetch(oauth_applications_id_column),
          account_id: membership.account_id,
          household_membership_id: membership.id,
          person_id: grant[:person_id] || membership.person_id,
          scopes: grant[oauth_grants_scopes_column] || scopes.join(oauth_scope_separator)
        }
      end

      def audit_auth_token(token_type, action, metadata = {})
        audit_account = Account.find_by(id: account_id)
        return unless audit_account

        AuthTokenAuditLogger.new.record(
          account: audit_account,
          token_type: token_type,
          action: action,
          metadata: metadata,
          context: {
            whodunnit: audit_account.person&.user&.id,
            ip: request.ip,
            request_id: rails_controller_instance&.request&.request_id
          }
        )
      end

      def password_complex_enough?(password)
        return true if password.match?(/\d/) && password.match?(/[^a-zA-Z\d]/)

        set_password_requirement_error_message(:password_simple, 'requires one number and one special character')
        false
      end

      def webauthn_user_verification
        if current_route == :webauthn_login || @webauthn_login
          'required'
        else
          super
        end
      end

      def webauthn_key_insert_hash(webauthn_credential)
        super.merge(
          nickname: submitted_webauthn_key_nickname,
          created_at: Time.current,
          updated_at: Time.current
        )
      end

      def submitted_webauthn_key_nickname
        nickname = webauthn_key_nickname_param
        nickname.presence&.slice(0, 255) || next_webauthn_key_nickname
      end

      def webauthn_key_nickname_param
        param_or_nil('nickname').to_s.strip
      rescue NoMethodError
        ''
      end

      def next_webauthn_key_nickname
        index = AccountWebauthnKey.where(account_id: webauthn_account_id).count + 1
        "Passkey #{index}"
      end

      def possible_authentication_methods
        methods = super
        return methods if methods.intersect?(%w[totp webauthn sms_code])

        methods - ['recovery_code']
      end

      def medtracker_app_url
        ENV.fetch('APP_URL') do
          raise KeyError, 'APP_URL is required in production' if Rails.env.production?

          request.base_url
        end.delete_suffix('/')
      end

      def create_household_for_account!(account_record, person, household: nil)
        household ||= create_owned_household(account_record, person)
        person.update!(household: household) if person.household_id != household.id
        membership = create_owner_membership(household, account_record, person)
        create_owner_person_grant(household, membership, person)
      end

      def create_owned_household(account_record, person)
        Household.create!(
          name: "#{person.name} Household",
          created_by_account: account_record
        )
      end

      def create_owner_membership(household, account_record, person)
        Households::AccessChange.new(
          actor_account: account_record,
          actor_membership: nil,
          request: rails_controller_instance&.request
        ).create_membership!(
          household: household,
          account: account_record,
          person: person,
          role: :owner,
          status: :active
        )
      end

      def create_owner_person_grant(household, membership, person)
        household_access_change(membership).create_grant!(
          household: household,
          household_membership: membership,
          person: person,
          access_level: :manage,
          relationship_type: :self,
          granted_by_membership: membership
        )
      end

      def accept_household_invitation!(account_record, person, invitation)
        membership = household_access_change(invitation.invited_by_membership).create_membership!(
          household: invitation.household,
          account: account_record,
          person: person,
          role: invitation.membership_role,
          status: :active
        )
        create_owner_person_grant(invitation.household, membership, person)
        apply_household_invitation_grants!(membership, person, invitation)
      end

      def apply_household_invitation_grants!(membership, person, invitation)
        invitation.household_invitation_grants.find_each do |grant|
          apply_household_invitation_grant!(membership, person, invitation, grant)
        end
      end

      def apply_household_invitation_grant!(membership, person, invitation, grant)
        relationship_type = carer_relationship_type_for_invitation_grant(grant.relationship_type)
        return create_invitation_manual_grant!(membership, invitation, grant) unless relationship_type

        CareDelegation::Assign.new(
          carer: person,
          patient: grant.person,
          relationship_type: relationship_type,
          access_level: grant.access_level,
          expires_at: grant.expires_at,
          granted_by_membership: invitation.invited_by_membership
        ).call
      end

      def create_invitation_manual_grant!(membership, invitation, grant)
        household_access_change(invitation.invited_by_membership).create_grant!(
          household: invitation.household,
          household_membership: membership,
          person: grant.person,
          access_level: grant.access_level,
          relationship_type: grant.relationship_type,
          expires_at: grant.expires_at,
          granted_by_membership: invitation.invited_by_membership
        )
      end

      def household_access_change(actor_membership)
        Households::AccessChange.new(
          actor_account: actor_membership&.account,
          actor_membership: actor_membership,
          request: rails_controller_instance&.request
        )
      end

      def carer_relationship_type_for_invitation_grant(relationship_type)
        case relationship_type.to_s
        when 'parent'
          'parent'
        when 'family_member'
          'family_member'
        when 'carer', 'professional'
          'professional_carer'
        end
      end

      def invite_only_registration_required?
        AppSettings.invite_only?
      end

      def invite_only_registration_message
        I18n.t('authentication.invite_only',
               default: 'Registration is by invitation only. Please contact an administrator.')
      end

      def account_active?
        account_record = Account.find_by(id: account&.[](:id))
        user = account_record&.person&.user

        account_record.nil? || user.nil? || user.active?
      end

      def inactive_account_message
        I18n.t('authentication.inactive_account',
               default: 'Your account has been deactivated. Please contact an administrator.')
      end
    end

    before_login do
      throw_error(login_param, inactive_account_message) unless account_active?
    end

    after_login do
      unless account_active?
        clear_session
        throw_error(login_param, inactive_account_message)
      end

      if param_or_nil('remember')
        remember_login
        audit_auth_token('remember_key', 'created')
      end

      id_token = omniauth_auth&.dig('credentials', 'id_token')
      next unless id_token

      session[:oidc_id_token] = id_token

      # Flag whether Zitadel satisfied MFA this session (amr claim, RFC 8176).
      # Only skip local 2FA enforcement when the IdP actually performed MFA.
      amr = Array(omniauth_auth.dig('extra', 'raw_info', 'amr'))
      session[:oidc_mfa_verified] = amr.intersect?(%w[mfa otp u2f hwk swk])

      account_record = Account.find_by(id: account_id)
      person = account_record&.person
      sync_zitadel_professional_title!(person, omniauth_auth) if person
    end

    # Extend user's remember period when remembered via a cookie
    extend_remember_deadline? true

    # Configure tables to use account_id instead of id (matches our migration)
    remember_id_column :account_id
    verify_account_id_column :account_id
    reset_password_id_column :account_id
    verify_login_change_id_column :account_id
    account_lockouts_id_column :account_id
    account_login_failures_id_column :account_id

    # Lock account after 5 failed login attempts
    max_invalid_logins 5
    # Unlock account after 30 minutes
    account_lockouts_deadline_interval(minutes: 30)

    # Track active sessions for session management
    # Session expires after 30 minutes of inactivity
    session_inactivity_deadline 30.minutes.to_i
    # Session expires after 24 hours regardless of activity
    session_lifetime_deadline 24.hours.to_i

    # TOTP issuer name shown in authenticator apps
    otp_issuer 'MedTracker'
    auto_remove_recovery_codes? true

    webauthn_rp_name 'MedTracker'
    webauthn_rp_id { URI.parse(medtracker_app_url).host }
    webauthn_origin { medtracker_app_url }
    webauthn_user_verification 'required'
    webauthn_authenticator_selection do
      {
        'residentKey' => 'required',
        'requireResidentKey' => true,
        'userVerification' => webauthn_user_verification
      }
    end
    webauthn_login_user_verification_additional_factor? true
    webauthn_login_error_flash { I18n.t('sessions.login.passkey_error') }
    webauthn_invalid_webauthn_id_message { I18n.t('sessions.login.passkey_error') }

    # Configure WebAuthn table column mappings for Rails conventions
    webauthn_keys_account_id_column :account_id
    webauthn_user_ids_account_id_column :account_id

    # Allow WebAuthn setup without requiring existing MFA
    # This enables users to add their first passkey without needing TOTP first
    two_factor_modifications_require_password? true

    # Allow setting up the first 2FA method without requiring existing 2FA
    # This fixes the chicken-and-egg problem where users can't set up TOTP
    # because they don't have any 2FA method yet
    two_factor_auth_required_redirect { two_factor_auth_path }
    two_factor_auth_return_to_requested_location? true
    after_two_factor_authentication do
      set_session_value(:privileged_action_mfa_verified_at, Time.current.to_i)
    end
    before_webauthn_auth do
      audit_auth_token('webauthn_verification', 'succeeded', outcome: 'success')
    end
    after_webauthn_auth_failure do
      audit_auth_token('webauthn_verification', 'failed', outcome: 'failure')
    end
    before_otp_setup_route do
      # Allow OTP setup if user has no 2FA methods configured yet
      # Only require 2FA auth if they already have a method set up
      next unless two_factor_authentication_setup?

      require_two_factor_authenticated
    end
    before_webauthn_setup_route do
      # Allow WebAuthn setup if user has no 2FA methods configured yet
      next unless two_factor_authentication_setup?

      require_two_factor_authenticated
    end
    before_recovery_codes_route do
      # Recovery codes require at least one 2FA method to be set up first
      # If no 2FA is set up, redirect to OTP setup with a message
      unless two_factor_authentication_setup?
        set_redirect_error_flash 'You need to set up an authenticator app or passkey before generating recovery codes'
        redirect otp_setup_path
      end
      # If 2FA is set up but not authenticated in this session, require auth
      require_two_factor_authenticated
    end

    # Configure OpenID Connect provider
    # Credentials from Rails credentials or environment variables
    oidc_issuer = Rails.application.credentials.dig(:oidc, :issuer_url) || ENV.fetch('OIDC_ISSUER_URL', nil)
    oidc_client_id = Rails.application.credentials.dig(:oidc, :client_id) || ENV.fetch('OIDC_CLIENT_ID', nil)
    oidc_client_secret = Rails.application.credentials.dig(:oidc,
                                                           :client_secret) || ENV.fetch('OIDC_CLIENT_SECRET', nil)
    app_url = ENV.fetch('APP_URL') do
      raise KeyError, 'APP_URL is required in production' if Rails.env.production?

      'http://localhost:3000'
    end.delete_suffix('/')

    if oidc_client_id.present? && oidc_issuer.present?
      omniauth_provider :openid_connect,
                        name: :oidc,
                        scope: %i[openid email profile],
                        response_type: :code,
                        uid_field: 'sub',
                        discovery: true,
                        issuer: oidc_issuer,
                        client_options: {
                          identifier: oidc_client_id,
                          secret: oidc_client_secret,
                          redirect_uri: ENV.fetch('OIDC_REDIRECT_URI', nil).presence ||
                                        "#{app_url}/auth/oidc/callback"
                        }
    end

    # Block the create-account page entirely when an administrator exists and no
    # valid invitation token is present. This prevents unauthenticated users from
    # even seeing the registration form in invite-only mode.
    before_create_account_route do
      next if param_or_nil('invitation_token').present?
      next unless invite_only_registration_required?

      set_notice_flash invite_only_registration_message
      redirect login_path
    end

    # Validate custom fields in the create account form.
    before_create_account do
      # Validate name
      name = param_or_nil('name')
      throw_error_status(422, 'name', 'must be present') if name.blank?

      # Validate date of birth
      date_of_birth_str = param_or_nil('date_of_birth')
      throw_error_status(422, 'date_of_birth', 'must be present') if date_of_birth_str.blank?

      begin
        date_of_birth = Date.parse(date_of_birth_str)
      rescue ArgumentError, TypeError
        throw_error_status(422, 'date_of_birth', 'must be a valid date')
      end

      if Person.new(date_of_birth: date_of_birth).age < 18
        throw_error_status(422, 'date_of_birth', 'Children must be added by a parent or carer.')
      end

      # Validate invitation token if present and lock down email
      if (token = param_or_nil('invitation_token'))
        @invitation = HouseholdInvitations::TokenResolver.call(token)
        throw_error_status(422, 'invitation_token', 'is invalid or expired') unless @invitation
        request.params[login_param] = @invitation.email
        account[login_column] = @invitation.email
      end

      if @invitation.nil? && invite_only_registration_required?
        throw_error_status(422, 'invitation_token', 'is required when registration is invitation-only')
      end
    end

    before_omniauth_create_account do
      next unless invite_only_registration_required?

      set_redirect_error_flash I18n.t('sessions.login.invite_only_oidc_notice',
                                      default: 'Single sign-on is reserved for invited accounts.')
      redirect login_path
    end

    # Perform additional actions after the account is created.
    after_create_account do
      date_of_birth = Date.parse(param('date_of_birth'))

      # Determine person_type based on age
      # Use Person's age calculation to avoid duplication
      temp_person = Person.new(date_of_birth: date_of_birth)
      age = temp_person.age
      person_type = age >= 18 ? :adult : :minor

      # Create associated Person and User records atomically
      # Ensures both are created or both fail together
      ActiveRecord::Base.transaction do
        account_record = Account.find(account_id)
        household = @invitation&.household || Household.create!(
          name: "#{param('name')} Household",
          created_by_account: account_record
        )
        TenantContext.with(
          account: account_record,
          household: household,
          request_id: rails_controller_instance&.request&.request_id
        ) do
          person = Person.create!(
            account: account_record,
            name: param('name'),
            date_of_birth: date_of_birth,
            email: account[:email],
            person_type: person_type,
            household: household
          )

          User.create!(
            person: person,
            email_address: account[:email],
            active: true
          )

          if @invitation
            accept_household_invitation!(account_record, person, @invitation)
          else
            create_household_for_account!(account_record, person, household: household)
          end

          @invitation&.update!(accepted_at: Time.current)
        end
      end
    end

    # Handle OAuth account creation - create Person and User records
    after_omniauth_create_account do
      # Get user info from OAuth provider
      auth_info = omniauth_info
      name = auth_info['name'] || auth_info['email'].split('@').first
      email = auth_info['email']

      # OAuth users are assumed to be adults (we don't have DOB from OAuth)
      # Use a sentinel DOB (100 years ago) to indicate missing data
      # Users should be prompted to update their profile with actual DOB
      ActiveRecord::Base.transaction do
        account_record = Account.find(account_id)
        household = Household.create!(
          name: "#{name} Household",
          created_by_account: account_record
        )
        person = Person.create!(
          account: account_record,
          name: name,
          email: email,
          person_type: :adult,
          date_of_birth: 100.years.ago.to_date, # Sentinel value - DOB unknown for OAuth users
          household: household
        )

        User.create!(
          person: person,
          email_address: email,
          active: true
        )

        create_household_for_account!(account_record, person, household: household)
      end
    end

    # Do additional cleanup after the account is closed.
    after_close_account do
      audit_auth_token('remember_key', 'revoked')
      # Nullify the account_id on the person but don't delete the person
      # This preserves medication history for compliance
      Person.where(account_id: account_id).find_each { |p| p.update!(account_id: nil) }
    end

    after_otp_setup do
      audit_auth_token('otp_key', 'created')
    end

    after_otp_disable do
      audit_auth_token('otp_key', 'revoked')
    end

    after_add_recovery_codes do
      audit_auth_token('recovery_codes', 'created')
    end

    after_webauthn_setup do
      audit_auth_token('webauthn_credential', 'created')
    end

    after_webauthn_remove do
      audit_auth_token('webauthn_credential', 'revoked')
    end

    after_remember do
      remember_value = param_or_nil(remember_param)
      if remember_value == remember_remember_param_value
        audit_auth_token('remember_key', 'created')
      elsif remember_value == remember_forget_param_value || remember_value == remember_disable_param_value
        audit_auth_token('remember_key', 'revoked')
      end
    end

    after_verify_account do
      audit_auth_token('verification_key', 'revoked')
    end

    after_reset_password do
      audit_auth_token('password_reset_key', 'revoked')
    end

    after_verify_login_change do
      audit_auth_token('login_change_key', 'revoked')
    end

    # Login redirect is routine, not an error — use notice instead of alert
    require_login_error_flash { I18n.t('authentication.login_required', default: 'Please login to continue') }

    # Render Phlex components directly for speed (no ERB indirection)
    auth_class_eval do
      def set_redirect_error_flash(message) # rubocop:disable Naming/AccessorMethodName
        return if message == require_login_error_flash

        super
      end

      def view(page, title)
        phlex_class = "Views::Rodauth::#{page.to_s.tr('-', '_').camelize}".safe_constantize
        if phlex_class
          set_title(title)
          is_modal = rails_controller_instance.request.headers['Turbo-Frame'] == 'modal'
          rails_controller_instance.render_to_string(phlex_class.new, layout: !is_modal)
        else
          super
        end
      end

      # Case-insensitive email auto-linking: Zitadel (and most IdPs) normalise
      # emails to lowercase, but accounts registered via the signup form may
      # have mixed-case addresses stored in the DB.  Rodauth's default lookup
      # is a plain WHERE clause which is case-sensitive in PostgreSQL, so a
      # stored address of "Jane@Hospital.org" would not match "jane@hospital.org"
      # from Zitadel — the account would appear non-existent and fall through to
      # account creation, where the invite-only gate blocks it.
      # account_table_ds already applies the status filter (excludes closed accounts).
      def _account_from_omniauth
        account_table_ds
          .where(Sequel.function(:lower, login_column) => omniauth_email.to_s.strip.downcase)
          .first
      end

      def sync_zitadel_professional_title!(person, auth_data)
        professional_title = zitadel_professional_title_for(auth_data)
        return unless professional_title && person.professional_title != professional_title

        return if person.update(professional_title: professional_title)

        Rails.logger.warn(
          "[OIDC] Professional title sync failed for #{person.id}: #{person.errors.full_messages.join(', ')}"
        )
      end

      def zitadel_professional_title_for(auth_data)
        zitadel_role_names(auth_data).find { |role| role.in?(%w[doctor nurse]) }
      end

      def zitadel_role_names(auth_data)
        raw_info = auth_data.dig('extra', 'raw_info') || {}
        return [] unless raw_info.key?('urn:zitadel:iam:org:project:roles')

        raw_info['urn:zitadel:iam:org:project:roles'].keys
      end
    end

    # Current.user is set in ApplicationController before_action instead of here

    # Redirect to dashboard after successful login
    login_redirect do
      account = Account.find_by(id: account_id)
      household = TenantContext.with(account: account, household: nil) { account&.first_active_household } if account
      household ? "/households/#{household.slug}/dashboard" : '/'
    end

    # Redirect to dashboard after account creation (since autologin is enabled)
    create_account_redirect { login_redirect }

    # Capture OIDC ID token before session is cleared on logout
    before_logout do
      @oidc_id_token_for_logout = session[:oidc_id_token]
    end

    # Redirect to OIDC provider's end_session endpoint for single sign-out
    after_logout do
      next unless @oidc_id_token_for_logout

      issuer = Rails.application.credentials.dig(:oidc, :issuer_url) || ENV.fetch('OIDC_ISSUER_URL', nil)
      next if issuer.blank?

      end_session_url = "#{issuer}/oidc/v1/end_session"
      app_url = medtracker_app_url
      redirect "#{end_session_url}?" \
               "id_token_hint=#{CGI.escape(@oidc_id_token_for_logout)}&" \
               "post_logout_redirect_uri=#{CGI.escape(app_url)}"
    end

    # Redirect to home page after logout.
    logout_redirect '/'

    # Redirect to wherever login redirects to after account verification.
    verify_account_redirect { login_redirect }

    # Redirect to login page after password reset.
    reset_password_redirect { login_path }

    # Change default deadlines for some actions.
    # This allows unverified users to login during the grace period in non-production
    verify_account_grace_period Rails.env.production? ? 0 : 7.days.to_i
    # reset_password_deadline_interval Hash[hours: 6]
    # verify_login_change_deadline_interval Hash[days: 2]
    # remember_deadline_interval Hash[days: 30]
  end
  # rubocop:enable Metrics/BlockLength
end
# rubocop:enable Metrics/ClassLength
