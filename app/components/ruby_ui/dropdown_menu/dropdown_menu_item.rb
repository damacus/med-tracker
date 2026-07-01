# frozen_string_literal: true

module RubyUI
  class DropdownMenuItem < Base
    def initialize(href: '#', as: :a, **attrs)
      @href = href
      @tag = as
      super(**attrs)
    end

    def view_template(&)
      if button_item?
        button(**attrs, &)
      else
        a(**attrs, &)
      end
    end

    private

    def default_attrs
      {
        href: button_item? ? nil : @href,
        type: button_item? ? :button : nil,
        role: 'menuitem',
        class: 'relative flex cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none transition-colors hover:bg-tertiary-container hover:text-on-tertiary-container focus:bg-tertiary-container focus:text-on-tertiary-container aria-selected:bg-tertiary-container aria-selected:text-on-tertiary-container data-[disabled]:pointer-events-none data-[disabled]:opacity-50',
        data_action: 'click->ruby-ui--dropdown-menu#close',
        data_ruby_ui__dropdown_menu_target: 'menuItem',
        tabindex: '-1',
        data_orientation: 'vertical'
      }.compact
    end

    def button_item?
      @tag == :button
    end
  end
end
