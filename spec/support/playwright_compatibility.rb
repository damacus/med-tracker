# frozen_string_literal: true

module PlaywrightCompatibility
  module InitScript
    def add_init_script(path: nil, script: nil)
      source =
        if path
          Playwright::JavaScript::SourceUrl.new(File.read(path), path).to_s
        elsif script
          script
        else
          raise ArgumentError, 'Either path or script parameter must be specified'
        end

      result = @channel.send_message_to_server_result('addInitScript', source: source)
      return nil unless result.respond_to?(:[])

      disposable = result['disposable']
      disposable ? Playwright::ChannelOwners::Disposable.from(disposable) : nil
    end
  end
end

Playwright::ChannelOwners::Page.prepend(PlaywrightCompatibility::InitScript)
Playwright::ChannelOwners::BrowserContext.prepend(PlaywrightCompatibility::InitScript)
