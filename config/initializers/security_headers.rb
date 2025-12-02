# frozen_string_literal: true

# Configure additional security headers for the application.
# Rails 8 provides X-Frame-Options, X-Content-Type-Options, and Referrer-Policy by default.
# This initializer adds Permissions-Policy to restrict browser features.

Rails.application.config.action_dispatch.default_headers.merge!(
  # Permissions-Policy restricts access to browser features
  # Disabling geolocation, camera, and microphone as they're not needed for this app
  'Permissions-Policy' => 'geolocation=(), camera=(), microphone=()'
)
