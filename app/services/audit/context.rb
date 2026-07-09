# frozen_string_literal: true

module Audit
  class Context
    SESSION_REFERENCE_SALT = 'med-tracker/audit-session-reference/v1'
    CONTEXT_KEYS = %i[
      actor_account_id actor_user_id actor_membership_id active_role permissions_version household_id
      support_access_session_id authentication_method session_reference request_id ip policy_class policy_query
    ].freeze

    class << self
      def start!(request:, credential:, **actor)
        Current.audit_context = build_context(
          request:,
          credential:,
          **actor
        )
        sync_paper_trail!
        current
      end

      def refresh!(account:, user:, membership: nil, support_access_session: nil)
        actor = actor_context(account:, user:, membership:, support_access_session:)
        Current.audit_context = current.merge(actor).compact
        sync_paper_trail!
        current
      end

      def authorized!(policy_class:, query:)
        Current.audit_context = current.merge(
          policy_class: policy_class.to_s,
          policy_query: query.to_s
        )
        sync_paper_trail!
        current
      end

      def current
        Current.audit_context || {}
      end

      def merge(overrides)
        current.merge(normalize_overrides(overrides)).compact
      end

      def clear!
        Current.audit_context = nil
        PaperTrail.request.controller_info = {}
        PaperTrail.request.whodunnit = nil
      end

      private

      def build_context(request:, credential:, **actor)
        actor_context(**actor).merge(
          authentication_context(request:, credential:),
          request_id: request.request_id,
          ip: request.remote_ip
        ).compact
      end

      def actor_context(account:, user:, membership: nil, support_access_session: nil)
        {
          actor_account_id: account&.id,
          actor_user_id: user&.id,
          actor_membership_id: membership&.id,
          active_role: active_role(account:, membership:, support_access_session:),
          permissions_version: membership&.permissions_version,
          household_id: membership&.household_id || support_access_session&.household_id,
          support_access_session_id: support_access_session&.id
        }
      end

      def active_role(account:, membership:, support_access_session:)
        return membership.role if membership
        return 'platform_admin' if account&.platform_admin && support_access_session

        'authenticated' if account
      end

      def authentication_context(request:, credential:)
        case credential
        when ApiSession
          { authentication_method: 'api_session', session_reference: "api_session:#{credential.id}" }
        when ApiAppToken
          { authentication_method: 'api_app_token', session_reference: "api_app_token:#{credential.id}" }
        when :web
          { authentication_method: 'web', session_reference: web_session_reference(request) }
        else
          { authentication_method: credential.to_s.presence }
        end
      end

      def web_session_reference(request)
        session_id = request.session.id.to_s
        return if session_id.blank?

        digest = OpenSSL::HMAC.hexdigest('SHA256', session_reference_key, session_id)
        "web:#{digest}"
      end

      def session_reference_key
        Rails.application.key_generator.generate_key(SESSION_REFERENCE_SALT, 32)
      end

      def normalize_overrides(overrides)
        values = overrides.to_h.symbolize_keys
        {
          actor_user_id: values.delete(:whodunnit),
          actor_membership_id: values[:actor_membership_id],
          household_id: values[:household_id],
          request_id: values[:request_id],
          ip: values[:ip]
        }.compact.merge(values.slice(*CONTEXT_KEYS))
      end

      def sync_paper_trail!
        context = current
        PaperTrail.request.whodunnit = context[:actor_user_id]&.to_s
        PaperTrail.request.controller_info = {
          ip: context[:ip],
          request_id: context[:request_id],
          household_id: context[:household_id],
          actor_membership_id: context[:actor_membership_id],
          audit_context: context
        }.compact
      end
    end
  end
end
