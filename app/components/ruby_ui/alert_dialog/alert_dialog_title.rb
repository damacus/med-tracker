# frozen_string_literal: true

module RubyUI
  class AlertDialogTitle < Base
    def view_template(&)
      h2(**attrs, &)
    end

    private

    def default_attrs
      {
        class: 'text-2xl font-semibold leading-tight tracking-tight text-foreground'
      }
    end
  end
end
