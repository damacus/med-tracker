# frozen_string_literal: true

module RubyUI
  module ActionStyleHelpers
    BASE_CLASSES = [
      'whitespace-nowrap inline-flex items-center justify-center rounded-shape-xl font-medium transition-colors',
      'disabled:pointer-events-none disabled:opacity-50',
      'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring',
      'aria-disabled:pointer-events-none aria-disabled:opacity-50 aria-disabled:cursor-not-allowed'
    ].freeze

    private

    def base_classes
      BASE_CLASSES
    end

    def size_classes
      if @icon
        case @size
        when :sm, :md, :lg then 'h-11 w-11 min-h-11 min-w-11'
        when :xl then 'h-12 w-12 min-h-12 min-w-12'
        end
      else
        case @size
        when :sm then 'px-3 py-2 min-h-[44px] text-sm'
        when :md then 'px-4 py-2 min-h-[44px] text-sm'
        when :lg then 'px-4 py-2 min-h-[44px] text-base'
        when :xl then 'px-6 py-3 min-h-12 text-base'
        end
      end
    end
  end
end
