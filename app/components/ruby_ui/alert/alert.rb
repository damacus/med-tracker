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
        'ring-warning/50 bg-warning/10 text-amber-900 [&>svg]:text-warning'
      when :success
        'ring-success/50 bg-success/10 text-green-900 [&>svg]:text-success'
      when :destructive
        'ring-destructive/50 bg-destructive/10 text-red-900 [&>svg]:text-destructive'
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
