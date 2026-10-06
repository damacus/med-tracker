# frozen_string_literal: true

module Components
  module Icons
    class Camera < Base
      def view_template
        svg(**attrs) do |s|
          s.path(d: 'M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3l-2.5-3z')
          s.circle(cx: '12', cy: '13', r: '3')
        end
      end
    end
  end
end
