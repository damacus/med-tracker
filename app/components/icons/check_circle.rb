# frozen_string_literal: true

module Components
  module Icons
    class CheckCircle < Base
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
            d: 'M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 ' \
               '12.586l7.293-7.293a1 1 0 011.414 0z',
            clip_rule: 'evenodd'
          )
        end
      end
    end
  end
end
