# frozen_string_literal: true

module RubyUI
  class ComboboxPopover < Base
    def view_template(&)
      div(**attrs, &)
    end

    private

    def default_attrs
      {
        class: 'absolute inset-auto m-0 rounded-shape-md border border-border/70 bg-popover shadow-elevation-3',
        role: 'popover',
        autofocus: true,
        popover: true,
        data: {
          ruby_ui__combobox_target: 'popover',
          action: %w[
            toggle->ruby-ui--combobox#handlePopoverToggle
            keydown.down->ruby-ui--combobox#keyDownPressed
            keydown.up->ruby-ui--combobox#keyUpPressed
            keydown.enter->ruby-ui--combobox#keyEnterPressed
            keydown.esc->ruby-ui--combobox#closePopover:prevent
            resize@window->ruby-ui--combobox#updatePopoverWidth
          ]
        }
      }
    end
  end
end
