# frozen_string_literal: true

module RubyUI
  class Button < Base
    BASE_CLASSES = [
      'whitespace-nowrap inline-flex items-center justify-center rounded-2xl font-medium transition-colors',
      'disabled:pointer-events-none disabled:opacity-50',
      'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring',
      'aria-disabled:pointer-events-none aria-disabled:opacity-50 aria-disabled:cursor-not-allowed'
    ].freeze

    def initialize(type: :button, variant: :primary, size: :md, icon: false, **attrs)
      @type = type
      @variant = variant.to_sym
      @size = size.to_sym
      @icon = icon
      super(**attrs)
    end

    def view_template(&)
      button(**attrs, &)
    end

    private

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

    def primary_classes
      [
        BASE_CLASSES,
        size_classes,
        'bg-primary text-on-primary',
        'hover:opacity-90 hover:scale-[1.02] active:scale-[0.98] transition-all'
      ]
    end

    def link_classes
      [
        BASE_CLASSES,
        size_classes,
        'text-primary underline-offset-4 font-bold',
        'hover:underline hover:opacity-80 transition-all'
      ]
    end

    def secondary_classes
      [
        BASE_CLASSES,
        size_classes,
        'bg-secondary text-secondary-foreground shadow-sm',
        'hover:opacity-80 hover:scale-[1.02] active:scale-[0.98] transition-all'
      ]
    end

    def destructive_classes
      [
        BASE_CLASSES,
        size_classes,
        'bg-error text-on-error',
        'hover:opacity-90 hover:scale-[1.02] active:scale-[0.98] transition-all'
      ]
    end

    def outline_classes
      [
        BASE_CLASSES,
        size_classes,
        'border border-input bg-background',
        'hover:bg-accent hover:text-accent-foreground hover:scale-[1.02] transition-all'
      ]
    end

    def destructive_outline_classes
      [
        BASE_CLASSES,
        size_classes,
        'border border-input bg-background',
        'text-error hover:bg-error-container hover:scale-[1.02] transition-all'
      ]
    end

    def success_outline_classes
      [
        BASE_CLASSES,
        size_classes,
        'border border-input bg-background',
        'text-success hover:bg-success-container hover:scale-[1.02] transition-all'
      ]
    end

    def ghost_classes
      [
        BASE_CLASSES,
        size_classes,
        'hover:bg-accent hover:text-accent-foreground'
      ]
    end

    def default_classes
      case @variant
      when :primary then primary_classes
      when :link then link_classes
      when :secondary then secondary_classes
      when :destructive then destructive_classes
      when :outline then outline_classes
      when :destructive_outline then destructive_outline_classes
      when :success_outline then success_outline_classes
      when :ghost then ghost_classes
      end
    end

    def default_attrs
      { type: @type, class: default_classes }
    end
  end
end
