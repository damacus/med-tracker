# frozen_string_literal: true

require 'ipaddr'
require 'uri'

class PushSubscriptionEndpointPolicy
  ALLOWED_HOSTS = %w[
    fcm.googleapis.com
    updates.push.services.mozilla.com
    web.push.apple.com
  ].freeze

  ALLOWED_HOST_SUFFIXES = %w[
    .notify.windows.com
    .push.apple.com
  ].freeze

  class << self
    def allowed?(endpoint)
      uri = URI.parse(endpoint.to_s)
      return false unless uri.is_a?(URI::HTTPS)
      return false if uri.host.blank? || uri.userinfo.present?

      host = uri.host.downcase
      allowed_host?(host) && !private_address?(host)
    rescue URI::InvalidURIError
      false
    end

    private

    def allowed_host?(host)
      ALLOWED_HOSTS.include?(host) ||
        ALLOWED_HOST_SUFFIXES.any? { |suffix| host.end_with?(suffix) }
    end

    def private_address?(host)
      address = IPAddr.new(host)

      address.loopback? || address.private? || address.link_local?
    rescue IPAddr::InvalidAddressError
      false
    end
  end
end
