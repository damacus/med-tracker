# frozen_string_literal: true

require 'sequel/core'

class RodauthMain < Rodauth::Rails::Auth
  # rubocop:disable Metrics/BlockLength -- Rodauth configuration DSL requires a single configure block
  configure do
    # List of authentication features that are loaded.
    enable :create_account, :verify_account, :verify_account_grace_period,
           :login, :logout, :remember, :lockout, :active_sessions,
           :reset_password, :change_password, :change_login, :verify_login_change,
           :close_account, :omniauth

    # See the Rodauth documentation for the list of available config options:
    # http://rodauth.jeremyevans.net/documentation.html

    # ==> General
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

    password_minimum_length 12
    # bcrypt has a maximum input length of 72 bytes, truncating any extra bytes.
    password_maximum_bytes 72 # add custom password complexity rules
    # Custom password complexity requirements (alternative to password_complexity feature).
    # password_meets_requirements? do |password|
    #   super(password) && password_complex_enough?(password)
    # end
    # auth_class_eval do
    #   def password_complex_enough?(password)
    #     return true if password.match?(/\d/) && password.match?(/[^a-zA-Z\d]/)
    #     set_password_requirement_error_message(:password_simple, "requires one number and one special character")
    #     false
    #   end
    # end

    after_login { remember_login if param_or_nil('remember') }

    # Extend user's remember period when remembered via a cookie
    extend_remember_deadline? true

    # Configure tables to use account_id instead of id (matches our migration)
    remember_id_column :account_id
    verify_account_id_column :account_id
    reset_password_id_column :account_id
    verify_login_change_id_column :account_id
    account_lockouts_id_column :account_id
    account_login_failures_id_column :account_id

    # ==> Lockout Configuration
    # Lock account after 5 failed login attempts
    max_invalid_logins 5
    # Unlock account after 30 minutes
    account_lockouts_deadline_interval(minutes: 30)

    # ==> Active Sessions Configuration
    # Track active sessions for session management
    # Session expires after 30 minutes of inactivity
    session_inactivity_deadline 30.minutes.to_i
    # Session expires after 24 hours regardless of activity
    session_lifetime_deadline 24.hours.to_i

    # ==> OmniAuth (Google OAuth)
    # Configure Google OAuth provider
    # Credentials should be stored in Rails credentials or environment variables
    if Rails.application.credentials.dig(:google, :client_id).present?
      omniauth_provider :google_oauth2,
                        Rails.application.credentials.google[:client_id],
                        Rails.application.credentials.google[:client_secret],
                        scope: 'email profile'
    elsif ENV['GOOGLE_CLIENT_ID'].present?
      omniauth_provider :google_oauth2,
                        ENV.fetch('GOOGLE_CLIENT_ID'),
                        ENV.fetch('GOOGLE_CLIENT_SECRET'),
                        scope: 'email profile'
    end

    # ==> Hooks
    # Validate custom fields in the create account form.
    before_create_account do
      # Validate name
      name = param_or_nil('name')
      throw_error_status(422, 'name', 'must be present') if name.blank?

      # Validate date of birth
      date_of_birth_str = param_or_nil('date_of_birth')
      throw_error_status(422, 'date_of_birth', 'must be present') if date_of_birth_str.blank?

      begin
        Date.parse(date_of_birth_str)
      rescue ArgumentError, TypeError
        throw_error_status(422, 'date_of_birth', 'must be a valid date')
      end
    end

    # Perform additional actions after the account is created.
    after_create_account do
      date_of_birth = Date.parse(param('date_of_birth'))

      # Determine person_type and role based on age
      # Use Person's age calculation to avoid duplication
      temp_person = Person.new(date_of_birth: date_of_birth)
      age = temp_person.age
      person_type = age >= 18 ? :adult : :minor
      user_role = age >= 18 ? :parent : :minor

      # Create associated Person and User records atomically
      # Ensures both are created or both fail together
      ActiveRecord::Base.transaction do
        person = Person.create!(
          account_id: account_id,
          name: param('name'),
          date_of_birth: date_of_birth,
          email: account[:email],
          person_type: person_type
        )

        # Create associated User record for authorization
        # New users default to 'parent' role (or 'minor' if under 18)
        User.create!(
          person: person,
          email_address: account[:email],
          role: user_role,
          active: true
        )
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
        person = Person.create!(
          account_id: account_id,
          name: name,
          email: email,
          person_type: :adult,
          date_of_birth: 100.years.ago.to_date # Sentinel value - DOB unknown for OAuth users
        )

        User.create!(
          person: person,
          email_address: email,
          role: :parent,
          active: true
        )
      end
    end

    # Do additional cleanup after the account is closed.
    after_close_account do
      # Nullify the account_id on the person but don't delete the person
      # This preserves medication history for compliance
      Person.where(account_id: account_id).find_each { |p| p.update!(account_id: nil) }
    end

    # ==> Views
    # Render Phlex components directly for speed (no ERB indirection)
    auth_class_eval do
      def view(page, title)
        phlex_class = "Views::Rodauth::#{page.to_s.tr('-', '_').camelize}".safe_constantize
        if phlex_class
          set_title(title)
          rails_controller_instance.render_to_string(phlex_class.new, layout: true)
        else
          super
        end
      end
    end

    # ==> Redirects
    # Current.user is set in ApplicationController before_action instead of here

    # Redirect to dashboard after successful login
    login_redirect '/dashboard'

    # Redirect to home page after logout.
    logout_redirect '/'

    # Redirect to wherever login redirects to after account verification.
    verify_account_redirect { login_redirect }

    # Redirect to login page after password reset.
    reset_password_redirect { login_path }

    # ==> Deadlines
    # Change default deadlines for some actions.
    # This allows unverified users to login during the grace period in non-production
    verify_account_grace_period Rails.env.production? ? 0 : 7.days.to_i
    # reset_password_deadline_interval Hash[hours: 6]
    # verify_login_change_deadline_interval Hash[days: 2]
    # remember_deadline_interval Hash[days: 30]
  end
  # rubocop:enable Metrics/BlockLength
end
