# frozen_string_literal: true

module Components
  module Icons
    class X < Base
      def view_template
        svg(**attrs) do |s|
          s.path(d: 'M18 6 6 18')
          s.path(d: 'm6 6 12 12')
        end
      end
    end
  end
end
