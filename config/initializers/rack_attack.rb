# frozen_string_literal: true

# Rack::Attack configuration for rate limiting and throttling
# See: https://github.com/rack/rack-attack

# Disable Rack::Attack in test environment to avoid rate limiting during tests
return if Rails.env.test?

class Rack::Attack
  ### Configure Cache ###
  # Use Rails cache for storing throttle data
  # In production, this should be backed by Redis or Memcached
  Rack::Attack.cache.store = ActiveSupport::Cache::MemoryStore.new

  ### Throttle Spammy Clients ###
  # Throttle all requests by IP (300 requests per 5 minutes)
  # This is a general safeguard against aggressive clients
  throttle('req/ip', limit: 300, period: 5.minutes, &:ip)

  ### Throttle Login Attempts ###
  # Throttle POST requests to /login by IP address
  # Key: "throttle:logins/ip:#{request.ip}"
  throttle('logins/ip', limit: 5, period: 20.seconds) do |req|
    req.ip if req.path == '/login' && req.post?
  end

  # Throttle POST requests to /login by email parameter
  # Key: "throttle:logins/email:#{normalized_email}"
  # This prevents credential stuffing attacks
  throttle('logins/email', limit: 5, period: 20.seconds) do |req|
    if req.path == '/login' && req.post?
      # Normalize email to prevent bypass via case variations
      req.params['email'].to_s.downcase.gsub(/\s+/, '')
    end
  end

  ### Throttle Account Creation ###
  # Prevent mass account creation
  throttle('accounts/ip', limit: 3, period: 1.minute) do |req|
    req.ip if req.path == '/create-account' && req.post?
  end

  ### Throttle Password Reset ###
  # Prevent password reset abuse
  throttle('password_reset/ip', limit: 5, period: 1.minute) do |req|
    req.ip if req.path == '/reset-password-request' && req.post?
  end

  throttle('password_reset/email', limit: 5, period: 1.hour) do |req|
    if req.path == '/reset-password-request' && req.post?
      req.params['email'].to_s.downcase.gsub(/\s+/, '')
    end
  end

  ### Custom Throttle Response ###
  # Return 429 Too Many Requests with Retry-After header
  self.throttled_responder = lambda do |request|
    match_data = request.env['rack.attack.match_data']
    now = match_data[:epoch_time]
    retry_after = match_data[:period] - (now % match_data[:period])

    [
      429,
      {
        'Content-Type' => 'text/plain',
        'Retry-After' => retry_after.to_s
      },
      ["Rate limit exceeded. Retry in #{retry_after} seconds.\n"]
    ]
  end

  ### Blocklist Known Bad Actors ###
  # Block requests from known bad IPs (can be populated from external sources)
  # blocklist('block bad IPs') do |req|
  #   BadIp.exists?(req.ip)
  # end

  ### Safelist Trusted Clients ###
  # Allow localhost and health check endpoints without throttling
  safelist('allow from localhost') do |req|
    req.ip == '127.0.0.1' || req.ip == '::1'
  end

  safelist('allow health checks') do |req|
    req.path == '/up' || req.path == '/health'
  end
end

# Enable Rack::Attack in the middleware stack
Rails.application.config.middleware.use Rack::Attack
