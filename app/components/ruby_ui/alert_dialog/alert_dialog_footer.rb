# frozen_string_literal: true

module RubyUI
  class AlertDialogFooter < Base
    def view_template(&)
      div(**attrs, &)
    end

    private

    def default_attrs
      {
        class: 'flex flex-col-reverse gap-2 sm:flex-row sm:justify-end sm:gap-0 sm:space-x-2 rtl:space-x-reverse'
      }
    end
  end
end
