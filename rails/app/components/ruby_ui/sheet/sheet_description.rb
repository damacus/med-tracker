# frozen_string_literal: true

module RubyUI
  class SheetDescription < Base
    def view_template(&)
      p(**attrs, &)
    end

    private

    def default_attrs
      {
        data: { ruby_ui_sheet_description: true },
        class: 'text-sm text-on-surface-variant'
      }
    end
  end
end
