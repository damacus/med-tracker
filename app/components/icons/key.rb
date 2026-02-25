# frozen_string_literal: true

module Components
  module Icons
    class Key < Base
      def view_template
        svg(**attrs) do |s|
          s.path(d: 'M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3m-3-3l-2.5-2.5')
        end
      end
    end
  end
end
