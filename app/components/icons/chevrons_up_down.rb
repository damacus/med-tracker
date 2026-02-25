# frozen_string_literal: true

module Components
  module Icons
    class ChevronsUpDown < Base
      def view_template
        svg(**attrs) do |s|
          s.path(d: 'm7 15 5 5 5-5')
          s.path(d: 'm7 9 5-5 5 5')
        end
      end
    end
  end
end
