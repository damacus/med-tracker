# frozen_string_literal: true

module RodauthMobileOauth
  private

  def valid_locked_oauth_grant(grant_params = nil)
    grant = super
    validate_mobile_account_grant!(grant)
    grant
  end

  def create_token_from_token(grant, update_params)
    validate_mobile_account_grant!(grant)
    super
  end

  def validate_mobile_account_grant!(grant)
    return unless grant[:client_kind] == 'mobile'
    return if OauthGrant.find(grant.fetch(:id)).active_for_account?

    redirect_response_error('invalid_grant')
  end

  def mobile_oauth_application?
    oauth_application&.[](:client_kind) == 'mobile'
  end

  def mobile_resource_owner_params
    started_at = mobile_authentication_started_at
    redirect login_path unless started_at

    { oauth_grants_account_id_column => account_id, client_kind: 'mobile',
      authenticated_at: started_at, last_used_at: Time.current,
      device_name: oauth_application.fetch(:name), created_at: Time.current, updated_at: Time.current }
  end

  def mobile_authentication_started_at
    active_sessions_ds
      .where(active_sessions_session_id_column => compute_hmacs(session[session_id_session_key]))
      .get(active_sessions_created_at_column)
  end

  def record_mobile_oauth_event(event_type, grant)
    Audit::VersionEvent.record!(
      item_type: 'Account', item_id: grant.fetch(oauth_grants_account_id_column),
      event: event_type.sub('smart_oauth.', 'mobile_oauth.'),
      request_id: rails_controller_instance&.request&.request_id,
      ip: request.ip,
      object: { oauth_application_id: grant[:oauth_application_id] || oauth_application.fetch(:id) },
      context: { actor_account_id: grant.fetch(oauth_grants_account_id_column) }
    )
  end
end
