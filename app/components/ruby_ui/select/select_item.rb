# frozen_string_literal: true

module RubyUI
  class SelectItem < Base
    def initialize(value: nil, **attrs)
      @value = value
      super(**attrs)
    end

    def view_template(&block)
      div(**attrs) do
        selected_icon
        block&.call
      end
    end

    private

    def selected_icon
      render ::Components::Icons::Check.new(class: 'invisible group-aria-selected:visible mr-2 h-4 w-4 flex-none')
    end

    def default_attrs
      {
        role: 'option',
        tabindex: '0',
        data_value: @value,
        aria_selected: 'false',
        data_orientation: 'vertical',
        class: [
          'item group relative flex cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none transition-colors',
          'focus:bg-accent focus:text-accent-foreground',
          'hover:bg-accent hover:text-accent-foreground',
          'disabled:pointer-events-none disabled:opacity-50',
          'aria-selected:bg-accent aria-selected:text-accent-foreground',
          'data-[disabled]:pointer-events-none data-[disabled]:opacity-50',
          'aria-disabled:pointer-events-none aria-disabled:opacity-50 aria-disabled:cursor-not-allowed'
        ],
        data: {
          controller: 'ruby-ui--select-item',
          action: 'click->ruby-ui--select#selectItem keydown.enter->ruby-ui--select#selectItem keydown.down->ruby-ui--select#handleKeyDown keydown.up->ruby-ui--select#handleKeyUp keydown.esc->ruby-ui--select#handleEsc',
          ruby_ui__select_target: 'item'
        }

      }
    end
  end
end
