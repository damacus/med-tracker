# frozen_string_literal: true

module Components
  module Icons
    class RefreshCw < Base
      def view_template
        svg(**attrs) do |s|
          s.path(d: 'M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8')
          s.path(d: 'M21 3v5h-5')
          s.path(d: 'M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16')
          s.path(d: 'M3 21v-5h5')
        end
      end
    end
  end
end
