# frozen_string_literal: true

# Be sure to restart your server when you modify this file.

# Define an application-wide content security policy.
# See the Securing Rails Applications Guide for more information:
# https://guides.rubyonrails.org/security.html#content-security-policy-header

Rails.application.configure do
  config.content_security_policy do |policy|
    policy.default_src :self
    policy.font_src    :self, :data
    policy.img_src     :self, :data, :https
    policy.object_src  :none
    policy.script_src  :self
    policy.style_src   :self
    policy.frame_ancestors :self
    policy.base_uri    :self
    policy.form_action :self
    policy.worker_src  :self, :blob

    # Allow connections to self and websockets for Turbo/ActionCable
    policy.connect_src :self, :wss
  end

  # Generate session nonces for permitted importmap and inline scripts.
  # Note: style-src is intentionally excluded from nonce directives because
  # Turbo injects inline styles at runtime (progress bar, drive transitions)
  # without access to the nonce, causing CSP violations.
  config.content_security_policy_nonce_generator = ->(request) { request.session.id.to_s }
  config.content_security_policy_nonce_directives = %w[script-src]

  # Report violations without enforcing the policy in development
  config.content_security_policy_report_only = Rails.env.development?
end
