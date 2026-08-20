# frozen_string_literal: true

module RubyUI
  class SheetTitle < Base
    def view_template(&)
      h3(**attrs, &)
    end

    private

    def default_attrs
      {
        data: { ruby_ui_sheet_title: true },
        class: 'text-lg font-semibold leading-none tracking-tight'
      }
    end
  end
end
