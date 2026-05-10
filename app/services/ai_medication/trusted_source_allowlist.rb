# frozen_string_literal: true

require "uri"

module AiMedication
  class TrustedSourceAllowlist
    Entry = Data.define(:name, :domains, :path_patterns, :seed_urls)

    def initialize(config_path: Rails.root.join("config/ai_medication_sources.yml"))
      @config_path = config_path
    end

    def allowed?(url)
      uri = parse_url(url)
      return false unless uri
      return false unless uri.is_a?(URI::HTTPS)

      entries.any? { |entry| host_allowed?(uri.host, entry.domains) && path_allowed?(uri.path, entry.path_patterns) }
    end

    def seed_urls
      entries.flat_map(&:seed_urls)
    end

    private

    attr_reader :config_path

    def entries
      @entries ||= begin
        payload = YAML.safe_load_file(config_path, aliases: false) || {}
        Array(payload.fetch("sources", [])).map do |source|
          Entry.new(
            name: source.fetch("name"),
            domains: Array(source.fetch("domains", [])),
            path_patterns: Array(source.fetch("path_patterns", [])).map { |pattern| Regexp.new(pattern) },
            seed_urls: Array(source.fetch("seed_urls", []))
          )
        end
      end
    end

    def parse_url(url)
      URI.parse(url.to_s)
    rescue URI::InvalidURIError
      nil
    end

    def host_allowed?(host, domains)
      normalized_host = host.to_s.downcase

      domains.any? do |domain|
        normalized_domain = domain.to_s.downcase
        normalized_host == normalized_domain || normalized_host.end_with?(".#{normalized_domain}")
      end
    end

    def path_allowed?(path, patterns)
      patterns.any? { |pattern| path.to_s.match?(pattern) }
    end
  end
end
