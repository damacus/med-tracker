# frozen_string_literal: true

module Platform
  class SupportAccessSessionsController < BaseController
    def create
      support_session = current_platform_admin.support_access_sessions.new(support_access_session_params)
      authorize support_session
      support_session.mfa_verified_at = privileged_action_mfa_verified_at
      support_session.request_id = request.request_id
      support_session.ip = request.remote_ip

      if support_session.valid?
        ActiveRecord::Base.transaction do
          support_session.save!
          record_support_access_event(support_session, 'support_access_session.started')
        end
        redirect_to platform_settings_path, notice: t('platform.support_access_sessions.created')
      else
        redirect_to platform_settings_path, alert: support_session.errors.full_messages.to_sentence
      end
    end

    def destroy
      support_session = current_platform_admin.support_access_sessions.find(params.expect(:id))
      authorize support_session
      ActiveRecord::Base.transaction do
        support_session.lock!
        if support_session.expires_at <= Time.current
          SupportAccessSessions::ExpiryProcessor.call(support_session: support_session)
        elsif support_session.ended_at.nil? && support_session.expired_at.nil?
          support_session.update!(ended_at: Time.current)
          record_support_access_event(support_session, 'support_access_session.ended')
        end
      end

      redirect_to platform_settings_path, notice: t('platform.support_access_sessions.ended')
    end

    private

    def current_platform_admin
      current_account.platform_admin
    end

    def support_access_session_params
      params.expect(support_access_session: %i[household_id reason])
    end

    def record_support_access_event(support_session, event_type)
      TenantContext.with(
        account: current_account,
        household: support_session.household,
        request_id: request.request_id
      ) do
        Audit::Event.record!(
          household: support_session.household,
          actor_account: current_account,
          event_type: event_type,
          request: request,
          metadata: {
            support_access_session_id: support_session.id
          }
        )
      end
    end
  end
end
