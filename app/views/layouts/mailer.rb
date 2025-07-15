# frozen_string_literal: true

module Views
  module Layouts
    # HTML email layout for the application.
    class Mailer < Views::Base
      def view_template(&block)
        doctype

        html do
          head do
            meta 'http-equiv': 'Content-Type', content: 'text/html; charset=utf-8'
            style do
              plain '/* Email styles need to be inline */'
            end
          end

          body(&block)
        end
      end
    end
  end
end
