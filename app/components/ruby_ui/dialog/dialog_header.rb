# frozen_string_literal: true

module RubyUI
  class DialogHeader < Base
    def view_template(&)
      div(**attrs, &)
    end

    private

    def default_attrs
      {
        class: 'flex flex-col gap-2 border-b border-border/60 bg-popover px-8 pb-4 pt-8 text-center sm:text-left rtl:sm:text-right'
      }
    end
  end
end
