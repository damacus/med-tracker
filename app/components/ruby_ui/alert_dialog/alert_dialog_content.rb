# frozen_string_literal: true

module RubyUI
  class AlertDialogContent < Base
    def view_template(&)
      template(data: { 'ruby-ui--alert-dialog-target': 'content' }) do
        div(data: { controller: 'ruby-ui--alert-dialog' }) do
          background
          container(&)
        end
      end
    end

    def background
      div(
        data_state: 'open',
        class: 'fixed inset-0 z-50 bg-foreground/10 backdrop-blur-[1.5px] data-[state=open]:animate-in pointer-events-auto',
        data_aria_hidden: 'true',
        aria_hidden: 'true'
      )
    end

    def container(&)
      div(
        role: 'alertdialog',
        data_state: 'open',
        class: 'fixed left-[50%] top-[50%] z-50 flex max-h-screen w-full max-w-lg translate-x-[-50%] translate-y-[-50%] flex-col gap-4 overflow-y-auto border border-border/70 bg-popover p-8 text-foreground shadow-elevation-5 duration-200 data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95 sm:rounded-shape-xl md:w-full pointer-events-auto',
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
