# frozen_string_literal: true

module Components
  module Icons
    class ChevronRight < Base
      DEFAULT_PATH = 'm9 18 6-6-6-6'

      def initialize(path: DEFAULT_PATH, stroke_width: nil, **attrs)
        @path = path
        @stroke_width = stroke_width
        super(**attrs)
      end

      def view_template
        svg(**attrs) do |s|
          s.path(d: path)
        end
      end

      private

      attr_reader :path, :stroke_width

      def default_attrs
        return super unless stroke_width

        super.merge(stroke_width: stroke_width)
      end
    end
  end
end
