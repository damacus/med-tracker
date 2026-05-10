# frozen_string_literal: true

module AiMedication
  module Tools
    class FetchMedicationSource < (defined?(RubyLLM::Tool) ? RubyLLM::Tool : Object)
      if respond_to?(:description)
        description "Fetches a trusted medication guidance source after URL allowlist validation"
      end

      if respond_to?(:params)
        params do
          string :url, description: "Trusted medication guidance URL"
        end
      end

      def initialize(allowlist: TrustedSourceAllowlist.new, client: SourcePageClient.new)
        super()
        @allowlist = allowlist
        @client = client
      end

      def execute(url:)
        return {error: "source_not_allowed"} unless @allowlist.allowed?(url)

        page = @client.fetch(url)
        {url: page.url, title: page.title, text: page.text}
      rescue StandardError => e
        {error: "source_fetch_failed", message: e.message}
      end
    end
  end
end
