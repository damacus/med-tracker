# frozen_string_literal: true

require 'sequel/core'

class RodauthMain < Rodauth::Rails::Auth
  configure do
    # List of authentication features that are loaded.
    enable :create_account, :verify_account, :verify_account_grace_period,
           :login, :logout, :remember,
           :reset_password, :change_password, :change_login, :verify_login_change,
           :close_account

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

    # ==> Hooks
    # Validate custom fields in the create account form.
    # before_create_account do
    #   throw_error_status(422, "name", "must be present") if param("name").empty?
    # end

    # Perform additional actions after the account is created.
    # after_create_account do
    #   Profile.create!(account_id: account_id, name: param("name"))
    # end

    # Do additional cleanup after the account is closed.
    # after_close_account do
    #   Profile.find_by!(account_id: account_id).destroy
    # end

    # ==> Views
    # Rodauth will automatically use templates in app/views/rodauth/

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
end
