# frozen_string_literal: true

class RodauthApp < Rodauth::Rails::App
  configure RodauthMain

  route do |r|
    rodauth.load_memory # autologin remembered users

    r.get 'webauthn-remove' do
      rodauth.require_two_factor_authenticated
      rodauth.view 'webauthn-remove', 'Remove Passkey'
    end

    r.rodauth
  rescue ActionController::InvalidAuthenticityToken
    r.session.clear
    flash[:alert] = I18n.t('authentication.session_expired', default: 'Your session expired. Please sign in again.')
    r.redirect rodauth.login_path
  end
end
