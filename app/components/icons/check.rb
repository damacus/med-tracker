# frozen_string_literal: true

module Components
  module Icons
    class Check < Base
      def view_template
        svg(**merged_attrs) do |s|
          s.path(d: 'M20 6 9 17l-5-5')
        end
      end
    end
  end
end
