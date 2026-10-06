# frozen_string_literal: true

module Components
  module Icons
    class Activity < Base
      def view_template
        svg(**attrs) do |s|
          s.path(d: 'M22 12h-4l-3 9L9 3l-3 9H2')
        end
      end
    end
  end
end
