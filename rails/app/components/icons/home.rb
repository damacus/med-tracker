# frozen_string_literal: true

module Components
  module Icons
    class Home < Base
      def view_template
        svg(**attrs) do |s|
          s.path(d: 'm3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z')
          s.path(d: 'M9 22V12h6v10')
        end
      end
    end
  end
end
