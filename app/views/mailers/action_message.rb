# frozen_string_literal: true

module Views
  module Mailers
    class ActionMessage < Views::Base
      def initialize(title:, instruction:, button_text:, button_url:, notice:)
        super()
        @title = title
        @instruction = instruction
        @button_text = button_text
        @button_url = button_url
        @notice = notice
      end

      def view_template
        render Layout.new do
          h1(class: 'mailer-title') { @title }
          p(class: 'mailer-copy') { @instruction }
          p(class: 'mailer-action-wrap') do
            a(href: @button_url, class: 'mailer-button') { @button_text }
          end
          p(class: 'mailer-note') { @notice }
        end
      end
    end
  end
end
