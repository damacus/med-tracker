# frozen_string_literal: true

module RubyUI
  class DropdownMenuContent < Base
    def view_template(&)
      div(**wrapper_attrs) do
        div(**attrs, &)
      end
    end

    private

    def default_attrs
      {
        data: {
          state: :open
        },
        class: 'z-50 min-w-[8rem] w-56 rounded-shape-md border border-border/70 bg-popover p-1 text-foreground shadow-elevation-3 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2'
      }
    end

    def wrapper_attrs
      {
        class: [
          'z-50 hidden group-[.is-absolute]/dropdown-menu:absolute',
          'group-[.is-fixed]/dropdown-menu:fixed',
          'w-max top-0 left-0'
        ],
        data: { ruby_ui__dropdown_menu_target: 'content' }
      }
    end
  end
end
