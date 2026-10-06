# frozen_string_literal: true

module RubyUI
  class Button < Base
    include ActionStyleHelpers

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

    def primary_classes
      [
        base_classes,
        size_classes,
        'bg-primary text-primary-foreground',
        'hover:opacity-90 hover:scale-[1.02] active:scale-[0.98] transition-all'
      ]
    end

    def link_classes
      [
        base_classes,
        size_classes,
        'text-primary underline-offset-4 font-bold',
        'hover:underline hover:opacity-80 transition-all'
      ]
    end

    def secondary_classes
      [
        base_classes,
        size_classes,
        'bg-secondary-container text-on-secondary-container shadow-sm',
        'hover:opacity-80 hover:scale-[1.02] active:scale-[0.98] transition-all'
      ]
    end

    def destructive_classes
      [
        base_classes,
        size_classes,
        'bg-error text-on-error',
        'hover:opacity-90 hover:scale-[1.02] active:scale-[0.98] transition-all'
      ]
    end

    def outline_classes
      [
        base_classes,
        size_classes,
        'border border-outline bg-background',
        'hover:bg-tertiary-container hover:text-on-tertiary-container hover:scale-[1.02] transition-all'
      ]
    end

    def destructive_outline_classes
      [
        base_classes,
        size_classes,
        'border border-outline bg-background',
        'text-error hover:bg-error-container hover:scale-[1.02] transition-all'
      ]
    end

    def success_outline_classes
      [
        base_classes,
        size_classes,
        'border border-outline bg-background',
        'text-success hover:bg-success-container hover:scale-[1.02] transition-all'
      ]
    end

    def ghost_classes
      [
        base_classes,
        size_classes,
        'hover:bg-tertiary-container hover:text-on-tertiary-container'
      ]
    end

    def success_classes
      [
        base_classes,
        size_classes,
        'bg-success text-success-foreground shadow-sm',
        'hover:opacity-90 hover:scale-[1.02] active:scale-[0.98] transition-all'
      ]
    end

    def large_action_classes
      [
        'w-full py-6 rounded-shape-xl flex items-center justify-center font-bold transition-all',
        'shadow-elevation-1 hover:shadow-elevation-2 active:scale-[0.98]'
      ]
    end

    def default_classes
      case @variant
      when :primary then primary_classes
      when :link then link_classes
      when :secondary then secondary_classes
      when :destructive then destructive_classes
      when :success then success_classes
      when :large_action then large_action_classes
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
