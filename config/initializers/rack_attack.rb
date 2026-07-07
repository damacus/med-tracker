# frozen_string_literal: true

module Rack
  class Attack
    Rack::Attack.cache.store = ActiveSupport::Cache::MemoryStore.new
    MEDICATION_LOOKUP_PATH = %r{\A/households/[^/]+/medication-finder/search(?:\.[a-z]+)?\z}
    AI_MEDICATION_SUGGESTIONS_PATH = %r{\A/households/[^/]+/ai-medication-suggestions\z}
    MCP_PATH = '/mcp'

    throttle('req/ip', limit: 300, period: 5.minutes, &:ip)

    throttle('logins/ip', limit: 5, period: 20.seconds) do |req|
      req.ip if req.path == '/login' && req.post?
    end

    throttle('logins/email', limit: 5, period: 20.seconds) do |req|
      req.params['email'].to_s.downcase.gsub(/\s+/, '') if req.path == '/login' && req.post?
    end

    throttle('accounts/ip', limit: 3, period: 1.minute) do |req|
      req.ip if req.path == '/create-account' && req.post?
    end

    throttle('password_reset/ip', limit: 5, period: 1.minute) do |req|
      req.ip if req.path == '/reset-password-request' && req.post?
    end

    throttle('password_reset/email', limit: 5, period: 1.hour) do |req|
      req.params['email'].to_s.downcase.gsub(/\s+/, '') if req.path == '/reset-password-request' && req.post?
    end

    throttle('api/auth/login/ip', limit: 10, period: 1.minute) do |req|
      req.ip if req.path == '/api/v1/auth/login' && req.post?
    end

    throttle('api/auth/login/email', limit: 10, period: 1.minute) do |req|
      req.params['email'].to_s.downcase.gsub(/\s+/, '') if req.path == '/api/v1/auth/login' && req.post?
    end

    throttle('api/auth/refresh/ip', limit: 30, period: 1.minute) do |req|
      req.ip if req.path == '/api/v1/auth/refresh' && req.post?
    end

    throttle('api/auth/oidc_exchange/ip', limit: 10, period: 1.minute) do |req|
      req.ip if req.path == '/api/v1/auth/oidc_exchange' && req.post?
    end

    throttle('api/data_exports/ip', limit: 10, period: 1.minute) do |req|
      req.ip if req.get? && req.path.match?(%r{\A/api/v1/households/[^/]+/data_exports/})
    end

    throttle('api/sync_batches/ip', limit: 30, period: 1.minute) do |req|
      req.ip if req.post? && req.path.match?(%r{\A/api/v1/households/[^/]+/sync/batches\z})
    end

    throttle('api/medication_lookup/ip', limit: 60, period: 1.minute) do |req|
      req.ip if req.get? && req.path.match?(%r{\A/api/v1/households/[^/]+/medication_lookup\z})
    end

    throttle('api/ai_medication_suggestions/ip', limit: 10, period: 1.minute) do |req|
      req.ip if req.post? && req.path.match?(%r{\A/api/v1/households/[^/]+/ai_medication_suggestions\z})
    end

    throttle('api/admin/audit_logs/ip', limit: 100, period: 1.minute) do |req|
      req.ip if req.get? && req.path.match?(%r{\A/api/v1/households/[^/]+/admin/audit_logs})
    end

    throttle('mcp/ip', limit: 60, period: 1.minute) do |req|
      req.ip if req.path == MCP_PATH
    end

    throttle('admin/audit_logs/ip', limit: 100, period: 1.minute) do |req|
      req.ip if req.path.match?(%r{\A/households/[^/]+/admin/audit_logs})
    end

    throttle('admin/audit_logs/user', limit: 200, period: 1.minute) do |req|
      if req.path.match?(%r{\A/households/[^/]+/admin/audit_logs})
        session = req.env['rack.session']
        session && session['account_id']
      end
    end

    throttle('medication_lookup/ip', limit: 60, period: 1.minute) do |req|
      req.ip if req.get? && req.path.match?(MEDICATION_LOOKUP_PATH)
    end

    throttle('medication_lookup/user', limit: 120, period: 1.hour) do |req|
      if req.get? && req.path.match?(MEDICATION_LOOKUP_PATH)
        session = req.env['rack.session']
        session && session['account_id']
      end
    end

    throttle('ai_medication_suggestions/ip', limit: 10, period: 1.minute) do |req|
      req.ip if req.path.match?(AI_MEDICATION_SUGGESTIONS_PATH) && req.post?
    end

    throttle('ai_medication_suggestions/user', limit: 20, period: 1.hour) do |req|
      if req.path.match?(AI_MEDICATION_SUGGESTIONS_PATH) && req.post?
        session = req.env['rack.session']
        session && session['account_id']
      end
    end

    self.throttled_responder = lambda do |request|
      match_data = request.env['rack.attack.match_data']
      now = match_data[:epoch_time]
      retry_after = match_data[:period] - (now % match_data[:period])

      throttle_type = request.env['rack.attack.matched']
      ActiveSupport::Notifications.instrument('rack_attack.throttled', throttle: throttle_type, ip: request.ip)
      Rails.logger.warn("Rate limit exceeded: #{throttle_type} from IP #{request.ip}")

      [
        429,
        throttle_headers(request, match_data, retry_after),
        throttle_body(request, retry_after)
      ]
    end

    def self.throttle_headers(request, match_data, retry_after)
      headers = {
        'Retry-After' => retry_after.to_s,
        'ratelimit-limit' => match_data[:limit].to_s,
        'ratelimit-remaining' => '0',
        'ratelimit-reset' => (match_data[:epoch_time] + retry_after).to_s
      }
      headers['Content-Type'] = request.path.start_with?('/api/') ? 'application/json' : 'text/plain'
      headers
    end

    def self.throttle_body(request, retry_after)
      return ["Rate limit exceeded. Retry in #{retry_after} seconds.\n"] unless request.path.start_with?('/api/')

      [
        {
          error: {
            code: 'rate_limited',
            message: "Rate limit exceeded. Retry in #{retry_after} seconds."
          }
        }.to_json
      ]
    end

    unless Rails.env.test?
      safelist('allow from localhost') do |req|
        ['127.0.0.1', '::1'].include?(req.ip)
      end
    end

    safelist('allow health checks') do |req|
      ['/up', '/health'].include?(req.path)
    end
  end
end

Rack::Attack.enabled = !Rails.env.test?

Rails.application.config.middleware.use Rack::Attack
