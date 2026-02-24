# frozen_string_literal: true

module Components
  module Icons
    class User < Base
      def initialize(size: 16, **attrs)
        super
        @attrs[:viewBox] = '0 0 20 20'
        @attrs[:fill] = 'currentColor'
        @attrs[:stroke] = nil
        @attrs[:stroke_width] = nil
        @attrs[:stroke_linecap] = nil
        @attrs[:stroke_linejoin] = nil
      end

      def view_template
        svg(**merged_attrs.compact) do |s|
          s.path(
            fill_rule: 'evenodd',
            d: 'M10 9a3 3 0 100-6 3 3 0 000 6zm-7 9a7 7 0 1114 0H3z',
            clip_rule: 'evenodd'
          )
        end
      end
    end
  end
end
