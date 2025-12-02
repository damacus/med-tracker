# frozen_string_literal: true

Rails.application.config.action_dispatch.default_headers.merge!(
  'Permissions-Policy' => 'geolocation=(), camera=(), microphone=()'
)
