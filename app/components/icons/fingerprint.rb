# frozen_string_literal: true

module Components
  module Icons
    class Fingerprint < Base
      def view_template
        svg(**attrs) do |s|
          s.path(d: 'M12 10a2 2 0 0 0-2 2c0 1.02-.64 1.908-1.546 2.23')
          s.path(d: 'M8 10a4 4 0 0 1 8 0c0 .394.07.782.204 1.152')
          s.path(d: 'M11.712 16c.066.394.132.788.132 1.182')
          s.path(d: 'M21 12a9 9 0 0 0-9-9 8.98 8.98 0 0 0-6.733 3.033')
          s.path(d: 'M10 21a2 2 0 0 1-2-2 14.1 14.1 0 0 1 1.636-6.636')
          s.path(d: 'M13.824 19.141a4.98 4.98 0 0 0 .176-1.141')
          s.path(d: 'M16 21a6 6 0 0 0-6-6')
          s.path(d: 'M20 18a4 4 0 0 0-4-4')
        end
      end
    end
  end
end
