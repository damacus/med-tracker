# frozen_string_literal: true

class RodauthApp < Rodauth::Rails::App
  configure RodauthMain

  route do |r|
    rodauth.load_oauth_server_metadata_route
    rodauth.check_active_session
    rodauth.load_memory # autologin remembered users

    r.get 'webauthn-remove' do
      rodauth.require_two_factor_authenticated
      rodauth.view 'webauthn-remove', 'Remove Passkey'
    end

    r.rodauth
  rescue ActionController::InvalidAuthenticityToken
    pending_authorization = r.session[rodauth.login_redirect_session_key]
    restore_authorization = r.post? && r.path == rodauth.login_path &&
                            valid_mobile_authorization_return_path?(pending_authorization, rodauth.authorize_path)
    r.session.clear
    r.session[rodauth.login_redirect_session_key] = pending_authorization if restore_authorization
    flash[:alert] = I18n.t('authentication.session_expired', default: 'Your session expired. Please sign in again.')
    r.redirect rodauth.login_path
  end

  private

  def valid_mobile_authorization_return_path?(value, authorize_path)
    return false unless value.is_a?(String)

    uri = URI.parse(value)
    uri.relative? && uri.path == authorize_path && uri.query.present? &&
      [uri.host, uri.userinfo, uri.fragment].all?(&:nil?)
  rescue URI::InvalidURIError
    false
  end
end
