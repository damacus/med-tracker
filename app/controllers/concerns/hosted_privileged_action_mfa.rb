# frozen_string_literal: true

module HostedPrivilegedActionMfa
  extend ActiveSupport::Concern

  private

  def require_hosted_privileged_action_mfa
    return unless hosted_privileged_action_mfa_required?

    require_privileged_action_mfa
  end

  def require_privileged_action_mfa
    if privileged_action_mfa_satisfied?
      record_privileged_action_mfa_verified_at
      return
    end

    redirect_to_privileged_action_mfa
  end

  def privileged_action_mfa_verified_at
    timestamp = session[:privileged_action_mfa_verified_at] || session['privileged_action_mfa_verified_at']
    Time.zone.at(timestamp) if timestamp
  end

  def hosted_privileged_action_mfa_required?
    ActiveModel::Type::Boolean.new.cast(ENV.fetch('HOSTED_ADMIN_MFA_REQUIRED', nil))
  end

  def privileged_action_mfa_satisfied?
    return true if ApiAuthState.web_session_oidc_mfa_verified?(session)
    return false unless ApiAuthState.mfa_configured?(current_account)

    ApiAuthState.web_session_mfa_method_present?(session)
  end

  def redirect_to_privileged_action_mfa
    if ApiAuthState.mfa_configured?(current_account)
      redirect_to rodauth.two_factor_auth_path,
                  alert: t('authentication.privileged_mfa_required.verify'),
                  status: :see_other
    else
      redirect_to privileged_action_mfa_setup_path,
                  alert: t('authentication.privileged_mfa_required.setup'),
                  status: :see_other
    end
  end

  def record_privileged_action_mfa_verified_at
    session[:privileged_action_mfa_verified_at] ||= Time.current.to_i
  end

  def privileged_action_mfa_setup_path
    return profile_path if Current.household || default_household_for_urls

    rodauth.otp_setup_path
  end
end
