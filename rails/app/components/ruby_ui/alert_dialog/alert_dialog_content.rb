# frozen_string_literal: true

module RubyUI
  class AlertDialogContent < Base
    def view_template(&)
      dialog(
        role: 'alertdialog',
        aria: {
          modal: 'true'
        },
        tabindex: '-1',
        data: {
          ruby_ui__alert_dialog_target: 'content',
          action: 'cancel->ruby-ui--alert-dialog#dismiss keydown->ruby-ui--alert-dialog#trapFocus'
        },
        data_state: 'open',
        hidden: true,
        class: 'fixed left-[50%] top-[50%] z-50 flex max-h-screen w-full max-w-lg translate-x-[-50%] translate-y-[-50%] flex-col gap-4 overflow-y-auto border border-border/70 bg-popover p-8 text-foreground shadow-elevation-5 duration-200 data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95 sm:rounded-shape-xl md:w-full backdrop:bg-foreground/10 backdrop:backdrop-blur-[1.5px]',
        &
      )
    end

    private

    def default_attrs
      {
        data: {
          ruby_ui__alert_dialog_target: 'content'
        }
      }
    end
  end
end
