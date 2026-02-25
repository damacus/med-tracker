# frozen_string_literal: true

module Components
  module Icons
    class ChevronRight < Base
      def view_template
        svg(**attrs) do |s|
          s.path(d: 'm9 18 6-6-6-6')
        end
      end
    end
  end
end
