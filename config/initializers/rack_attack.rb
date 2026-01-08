# frozen_string_literal: true

return if Rails.env.test?

class Rack::Attack
  Rack::Attack.cache.store = ActiveSupport::Cache::MemoryStore.new

  throttle('req/ip', limit: 300, period: 5.minutes, &:ip)

  throttle('logins/ip', limit: 5, period: 20.seconds) do |req|
    req.ip if req.path == '/login' && req.post?
  end

  throttle('logins/email', limit: 5, period: 20.seconds) do |req|
    if req.path == '/login' && req.post?
      req.params['email'].to_s.downcase.gsub(/\s+/, '')
    end
  end

  throttle('accounts/ip', limit: 3, period: 1.minute) do |req|
    req.ip if req.path == '/create-account' && req.post?
  end

  throttle('password_reset/ip', limit: 5, period: 1.minute) do |req|
    req.ip if req.path == '/reset-password-request' && req.post?
  end

  throttle('password_reset/email', limit: 5, period: 1.hour) do |req|
    if req.path == '/reset-password-request' && req.post?
      req.params['email'].to_s.downcase.gsub(/\s+/, '')
    end
  end

  throttle('admin/audit_logs/ip', limit: 100, period: 1.minute) do |req|
    req.ip if req.path.start_with?('/admin/audit_logs')
  end

  throttle('admin/audit_logs/user', limit: 200, period: 1.minute) do |req|
    if req.path.start_with?('/admin/audit_logs')
      session = req.env['rack.session']
      session && session['account_id']
    end
  end

  self.throttled_responder = lambda do |request|
    match_data = request.env['rack.attack.match_data']
    now = match_data[:epoch_time]
    retry_after = match_data[:period] - (now % match_data[:period])

    throttle_type = request.env['rack.attack.matched']
    Rails.logger.warn("Rate limit exceeded: #{throttle_type} from IP #{request.ip}")

    [
      429,
      {
        'Content-Type' => 'text/plain',
        'Retry-After' => retry_after.to_s
      },
      ["Rate limit exceeded. Retry in #{retry_after} seconds.\n"]
    ]
  end

  safelist('allow from localhost') do |req|
    req.ip == '127.0.0.1' || req.ip == '::1'
  end

  safelist('allow health checks') do |req|
    req.path == '/up' || req.path == '/health'
  end
end

Rails.application.config.middleware.use Rack::Attack
