# frozen_string_literal: true

module Components
  module Icons
    class Pencil < Base
      def view_template
        svg(**attrs) do |s|
          s.path(d: 'M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z')
          s.path(d: 'm15 5 4 4')
        end
      end
    end
  end
end
