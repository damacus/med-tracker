# frozen_string_literal: true

module SupportAccessSessions
  class ExpiryProcessor
    EVENT_TYPE = 'support_access_session.expired'

    class << self
      def call(support_session: nil)
        new(support_session: support_session).call
      end
    end

    def initialize(support_session: nil)
      @support_session = support_session
    end

    def call
      expired_sessions.count { |support_session| expire(support_session) }
    end

    private

    def expired_sessions
      return [@support_session] if @support_session

      SupportAccessSession
        .where(ended_at: nil, expired_at: nil)
        .where(expires_at: ..Time.current)
        .includes(platform_admin: :account)
        .find_each
    end

    def expire(support_session)
      TenantContext.with(
        account: support_session.platform_admin.account,
        household: support_session.household
      ) do
        support_session.with_lock do
          next false unless naturally_expired?(support_session)

          support_session.update!(expired_at: Time.current)
          record_expiry(support_session)
          true
        end
      end
    end

    def naturally_expired?(support_session)
      support_session.ended_at.nil? && support_session.expired_at.nil? && support_session.expires_at <= Time.current
    end

    def record_expiry(support_session)
      Audit::Event.record!(
        household: support_session.household,
        actor_account: support_session.platform_admin.account,
        event_type: EVENT_TYPE,
        metadata: {
          support_access_session_id: support_session.id,
          expired_at: support_session.expires_at.iso8601,
          outcome: 'expired'
        }
      )
    end
  end
end
