# frozen_string_literal: true

module RubyUI
  class DialogTitle < Base
    def view_template(&)
      h3(**attrs, &)
    end

    private

    def default_attrs
      {
        data: { ruby_ui_dialog_title: true },
        class: 'text-3xl font-semibold leading-tight tracking-tight text-foreground'
      }
    end
  end
end
