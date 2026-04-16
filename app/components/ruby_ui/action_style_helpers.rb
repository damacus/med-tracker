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
        when :sm then 'h-8 w-8 min-h-[32px] min-w-[32px]'
        when :md then 'h-9 w-9 min-h-[36px] min-w-[36px]'
        when :lg then 'h-10 w-10 min-h-[40px] min-w-[40px]'
        when :xl then 'h-12 w-12 min-h-[48px] min-w-[48px]'
        end
      else
        case @size
        when :sm then 'px-3 py-1.5 h-8 min-h-[32px] text-xs'
        when :md then 'px-4 py-2 h-9 min-h-[36px] text-sm'
        when :lg then 'px-4 py-2 h-10 min-h-[40px] text-base'
        when :xl then 'px-6 py-3 h-12 min-h-[48px] text-base'
        end
      end
    end
  end
end
