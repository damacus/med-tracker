# frozen_string_literal: true

module RubyUI
  # Alert component for displaying flash messages and notifications
  class Alert < Base
    def initialize(variant: nil, **attrs)
      @variant = variant
      super(**attrs) # must be called after variant is set
    end

    def view_template(&)
      div(**attrs, &)
    end

    private

    def colors
      case @variant
      when nil
        'ring-border bg-muted/20 text-foreground [&>svg]:opacity-80'
      when :warning
        'ring-warning/50 bg-warning-container text-on-warning-container [&>svg]:text-warning'
      when :success
        'ring-success/50 bg-success-container text-on-success-container [&>svg]:text-success'
      when :destructive
        'ring-error/50 bg-error-container text-on-error-container [&>svg]:text-error'
      end
    end

    def default_attrs
      base_classes = 'backdrop-blur relative w-full ring-1 ring-inset rounded-lg px-4 py-4 text-sm ' \
                     '[&>svg+div]:translate-y-[-3px] [&>svg]:absolute [&>svg]:start-4 [&>svg]:top-4 [&>svg~*]:ps-8'
      {
        class: [base_classes, colors],
        role: 'alert'
      }
    end
  end
end
