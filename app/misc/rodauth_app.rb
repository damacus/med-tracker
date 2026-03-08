# frozen_string_literal: true

class RodauthApp < Rodauth::Rails::App
  # primary configuration
  configure RodauthMain

  # secondary configuration
  # configure RodauthAdmin, :admin

  route do |r|
    rodauth.load_memory # autologin remembered users

    # Handle GET request for passkey removal confirmation
    r.get 'webauthn-remove' do
      rodauth.require_two_factor_authenticated
      rodauth.view 'webauthn-remove', 'Remove Passkey'
    end

    r.rodauth # route rodauth requests
  rescue ActionController::InvalidAuthenticityToken
    r.session.clear
    flash[:alert] = I18n.t('authentication.session_expired', default: 'Your session expired. Please sign in again.')
    r.redirect rodauth.login_path

    # ==> Authenticating requests
    # Call `rodauth.require_account` for requests that you want to
    # require authentication for. For example:
    #
    # # authenticate /dashboard/* and /account/* requests
    # if r.path.start_with?("/dashboard") || r.path.start_with?("/account")
    #   rodauth.require_account
    # end

    # ==> Secondary configurations
    # r.rodauth(:admin) # route admin rodauth requests
  end
end
