# frozen_string_literal: true

module RubyUI
  class Link < Base
    include ActionStyleHelpers

    def initialize(href: '#', variant: :link, size: :md, icon: false, **attrs)
      @href = href
      @variant = normalize_variant(variant)
      @size = size.to_sym
      @icon = icon
      super(**attrs)
    end

    def view_template(&)
      a(href: @href, **attrs, &)
    end

    private

    def normalize_variant(variant)
      case variant.to_sym
      when :filled then :primary
      when :outlined then :outline
      when :text then :ghost
      else variant.to_sym
      end
    end

    def primary_classes
      [
        base_classes,
        size_classes,
        'bg-primary text-primary-foreground no-underline',
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
        'bg-secondary text-secondary-foreground no-underline shadow-sm',
        'hover:opacity-80 hover:scale-[1.02] active:scale-[0.98] transition-all'
      ]
    end

    def destructive_classes
      [
        base_classes,
        size_classes,
        'bg-error text-on-error no-underline',
        'hover:opacity-90 hover:scale-[1.02] active:scale-[0.98] transition-all'
      ]
    end

    def outline_classes
      [
        base_classes,
        size_classes,
        'border border-outline bg-background no-underline',
        'hover:bg-tertiary-container hover:text-on-tertiary-container hover:scale-[1.02] transition-all'
      ]
    end

    def destructive_outline_classes
      [
        base_classes,
        size_classes,
        'border border-outline bg-background no-underline',
        'text-error hover:bg-error-container hover:scale-[1.02] transition-all'
      ]
    end

    def success_outline_classes
      [
        base_classes,
        size_classes,
        'border border-outline bg-background no-underline',
        'text-success hover:bg-success-container hover:scale-[1.02] transition-all'
      ]
    end

    def ghost_classes
      [
        base_classes,
        size_classes,
        'no-underline hover:bg-tertiary-container hover:text-on-tertiary-container'
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
      { class: default_classes }
    end
  end
end
