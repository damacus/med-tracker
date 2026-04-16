# frozen_string_literal: true

module Components
  module M3
  class Button < RubyUI::Button
    # Override BASE_CLASSES to ensure rounded-full and state-layer
    BASE_CLASSES = [
      'whitespace-nowrap inline-flex items-center justify-center rounded-shape-full font-medium transition-all state-layer',
      'disabled:pointer-events-none disabled:opacity-38',
      'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-secondary',
      'aria-disabled:pointer-events-none aria-disabled:opacity-38 aria-disabled:cursor-not-allowed'
    ].freeze

    # M3 Variant Mapping:
    # :filled   -> was :primary (but now rounded-full + state-layer)
    # :tonal    -> was :secondary (bg-secondary-container)
    # :elevated -> new (shadow + surface-container-low)
    # :outlined -> was :outline
    # :text     -> was :ghost

    def initialize(variant: :filled, **attrs)
      # Map M3 names to RubyUI internally if needed, or handle them here
      @m3_variant = variant.to_sym
      
      # Translate M3 variant to RubyUI base variant for inherited logic
      base_variant = case @m3_variant
                     when :filled then :primary
                     when :tonal then :secondary
                     when :text then :ghost
                     when :outlined then :outline
                     else @m3_variant
                     end
      
      super(variant: base_variant, **attrs)
    end

    def default_attrs
      { type: @type, class: default_classes }
    end

    private

    def primary_classes
      [
        BASE_CLASSES,
        size_classes,
        'bg-primary text-on-primary shadow-elevation-1 hover:shadow-elevation-2'
      ]
    end

    def secondary_classes
      [
        BASE_CLASSES,
        size_classes,
        'bg-secondary-container text-on-secondary-container shadow-elevation-1 hover:shadow-elevation-2'
      ]
    end

    def elevated_classes
      [
        BASE_CLASSES,
        size_classes,
        'bg-surface-container-low text-primary shadow-elevation-1 hover:shadow-elevation-2'
      ]
    end

    def outline_classes
      [
        BASE_CLASSES,
        size_classes,
        'border border-outline bg-transparent text-primary hover:bg-surface-container-low'
      ]
    end

    def ghost_classes
      [
        BASE_CLASSES,
        size_classes,
        'bg-transparent text-primary hover:bg-surface-container-low'
      ]
    end

    def destructive_classes
      [
        BASE_CLASSES,
        size_classes,
        'bg-error text-on-error shadow-elevation-1 hover:shadow-elevation-2'
      ]
    end

    def default_classes
      case @m3_variant
      when :filled then primary_classes
      when :tonal then secondary_classes
      when :elevated then elevated_classes
      when :outlined then outline_classes
      when :text then ghost_classes
      when :destructive then destructive_classes
      else super
      end
    end
  end
  end
end
