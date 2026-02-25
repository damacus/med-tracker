# frozen_string_literal: true

module Components
  module Icons
    class ArrowUp < Base
      def view_template
        svg(**attrs) do |s|
          s.path(d: 'm5 12 7-7 7 7')
          s.path(d: 'M12 19V5')
        end
      end
    end
  end
end
