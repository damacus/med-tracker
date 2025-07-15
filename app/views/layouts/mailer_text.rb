# frozen_string_literal: true

module Views
  module Layouts
    # Plain text email layout for the application.
    class MailerText < Views::Base
      def view_template
        yield
      end
    end
  end
end
