# frozen_string_literal: true

module Components
  module Icons
    class CheckCircle < Base
      def view_template
        svg(**attrs) do |s|
          s.circle(cx: "12", cy: "12", r: "10")
          s.path(d: "m9 12 2 2 4-4")
        end
      end
    end
  end
end
