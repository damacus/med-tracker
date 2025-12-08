# frozen_string_literal: true

module Components
  module Icons
    class Pill < Base
      def view_template
        svg(**merged_attrs) do |s|
          s.path(d: 'M10.5 20.5 10 21a2 2 0 0 1-2.828 0L4.343 18.172a2 2 0 0 1 0-2.828l.5-.5')
          s.path(d: 'm7 17-5-5')
          s.path(d: 'M13.5 3.5 14 3a2 2 0 0 1 2.828 0l2.829 2.828a2 2 0 0 1 0 2.829l-.5.5')
          s.path(d: 'm17 7 5 5')
          s.circle(cx: '12', cy: '12', r: '2')
        end
      end
    end
  end
end
