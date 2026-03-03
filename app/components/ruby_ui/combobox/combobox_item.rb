# frozen_string_literal: true

module RubyUI
  class ComboboxItem < Base
    def view_template(&)
      label(**attrs, &)
    end

    private

    def default_attrs
      {
        class: [
          'relative flex flex-row w-full text-wrap [&>span,&>div]:truncate gap-2 items-center rounded-sm py-1.5 px-2 text-sm outline-none cursor-pointer',
          'select-none has-[:checked]:bg-accent has-[:checked]:text-accent-foreground hover:bg-accent',
          '[&>svg]:pointer-events-none [&>svg]:size-4 [&>svg]:shrink-0 aria-[current=true]:bg-accent aria-[current=true]:ring aria-[current=true]:ring-offset-2',
          'has-[:disabled]:opacity-50 has-[:disabled]:cursor-not-allowed'
        ],
        role: 'option',
        data: {
          ruby_ui__combobox_target: 'item'
        }
      }
    end
  end
end
