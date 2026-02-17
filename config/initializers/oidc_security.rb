# frozen_string_literal: true

module OidcSecurity
  class ConfigurationError < StandardError; end

  module_function

  def configured?
    client_id = Rails.application.credentials.dig(:oidc, :client_id) || ENV.fetch('OIDC_CLIENT_ID', nil)
    issuer_url = Rails.application.credentials.dig(:oidc, :issuer_url) || ENV.fetch('OIDC_ISSUER_URL', nil)
    client_id.present? && issuer_url.present?
  end

  def validate_issuer_url!(url)
    raise ConfigurationError, 'OIDC issuer URL cannot be blank' if url.blank?

    begin
      uri = URI.parse(url)
    rescue URI::InvalidURIError
      raise ConfigurationError, "Invalid OIDC issuer URL: #{url}"
    end

    raise ConfigurationError, "Invalid OIDC issuer URL: #{url}" unless uri.is_a?(URI::HTTP)

    localhost = %w[localhost 127.0.0.1 ::1].include?(uri.host&.delete_prefix('[')&.delete_suffix(']'))
    raise ConfigurationError, "OIDC issuer URL must use HTTPS: #{url}" if uri.scheme == 'http' && !localhost
  end

  def validate_redirect_uri!(uri_string)
    raise ConfigurationError, 'OIDC redirect URI cannot be blank' if uri_string.blank?

    begin
      uri = URI.parse(uri_string)
    rescue URI::InvalidURIError
      raise ConfigurationError, "Invalid OIDC redirect URI: #{uri_string}"
    end

    raise ConfigurationError, "Invalid OIDC redirect URI: #{uri_string}" unless uri.is_a?(URI::HTTP)

    localhost = %w[localhost 127.0.0.1 ::1].include?(uri.host&.delete_prefix('[')&.delete_suffix(']'))
    raise ConfigurationError, "OIDC redirect URI must use HTTPS: #{uri_string}" if uri.scheme == 'http' && !localhost
  end

  def secret_not_in_source?
    source_files = Rails.root.glob('{app,config,lib}/**/*.{rb,yml,yaml,erb}').map(&:to_s)
    source_files.none? do |file|
      next if file.end_with?('.yml.enc')
      next if file.include?('oidc_security.rb')

      content = File.read(file)
      content.match?(/OIDC_CLIENT_SECRET\s*=\s*['"][^'"]+['"]/)
    end
  end
end

Rails.application.config.after_initialize do
  next unless OidcSecurity.configured?

  issuer_url = Rails.application.credentials.dig(:oidc, :issuer_url) || ENV.fetch('OIDC_ISSUER_URL', nil)
  redirect_uri = ENV.fetch('OIDC_REDIRECT_URI', nil)

  begin
    OidcSecurity.validate_issuer_url!(issuer_url)
  rescue OidcSecurity::ConfigurationError => e
    Rails.logger.warn("[OIDC] Configuration warning: #{e.message}")
  end

  begin
    OidcSecurity.validate_redirect_uri!(redirect_uri) if redirect_uri
  rescue OidcSecurity::ConfigurationError => e
    Rails.logger.warn("[OIDC] Configuration warning: #{e.message}")
  end

  Rails.logger.info('[OIDC] OpenID Connect provider configured')
end
