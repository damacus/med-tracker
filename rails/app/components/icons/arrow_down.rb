# frozen_string_literal: true

module Components
  module Icons
    class ArrowDown < Base
      def view_template
        svg(**attrs) do |s|
          s.path(d: 'm19 12-7 7-7-7')
          s.path(d: 'M12 5v14')
        end
      end
    end
  end
end
