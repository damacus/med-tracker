# frozen_string_literal: true

module RubyUI
  class Badge < Base
    SIZES = {
      sm: 'px-1.5 py-0.5 text-xs',
      md: 'px-2 py-1 text-xs',
      lg: 'px-3 py-1 text-sm'
    }.freeze

    COLORS = {
      primary: 'text-on-primary-container bg-primary-container ring-primary/20',
      secondary: 'text-secondary-foreground bg-secondary/10 ring-secondary/20',
      outline: 'text-foreground bg-background ring-border',
      destructive: 'text-on-error-container bg-error-container ring-error/20',
      success: 'text-on-success-container bg-success-container ring-success/20',
      warning: 'text-on-warning-container bg-warning-container ring-warning/20'
    }.freeze

    def initialize(variant: :primary, size: :md, **args)
      @variant = variant
      @size = size
      super(**args)
    end

    def view_template(&)
      span(**attrs, &)
    end

    private

    def default_attrs
      {
        class: ['inline-flex items-center rounded-md font-medium ring-1 ring-inset min-h-[24px] min-w-[24px]', SIZES[@size], COLORS[@variant]]
      }
    end
  end
end
