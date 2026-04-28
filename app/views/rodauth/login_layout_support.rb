# frozen_string_literal: true

module Views
  module Rodauth
    module LoginLayoutSupport
      private

      def login_page_layout(&)
        if view_context.request.headers['Turbo-Frame'] == 'modal'
          page_layout(&)
        else
          div(class: login_page_classes) do
            login_background
            div(class: login_content_classes, &)
          end
        end
      end

      def login_surface_attributes
        {
          data_login_surface: 'split-auth',
          class: 'grid w-full overflow-hidden rounded-3xl border border-outline-variant/70 bg-surface-container-lowest/95 shadow-elevation-4 backdrop-blur md:grid-cols-2 dark:bg-surface-container-low/90'
        }
      end

      def login_page_classes
        'relative min-h-screen overflow-hidden bg-[radial-gradient(circle_at_50%_-10%,rgb(240_249_255)_0%,rgb(248_250_252)_42%,rgb(226_232_240)_100%)] px-4 py-8 text-foreground sm:px-6 lg:px-8 dark:bg-[radial-gradient(circle_at_50%_-10%,rgb(15_23_42)_0%,rgb(2_8_23)_58%,rgb(1_5_14)_100%)]'
      end

      def login_content_classes
        'relative z-10 mx-auto flex min-h-[calc(100vh-4rem)] w-full max-w-6xl flex-col justify-center gap-5'
      end

      def brand_panel_classes
        'relative flex min-h-[42rem] flex-col gap-8 overflow-hidden border-b border-r border-outline-variant/60 p-8 sm:p-12'
      end

      def form_panel_classes
        'flex flex-col justify-center p-8 sm:p-12 lg:p-16'
      end

      def secondary_sign_in_button_classes
        'flex h-16 w-full items-center justify-between rounded-lg border border-outline-variant bg-surface-container-lowest px-2 pr-4 text-left font-bold text-foreground shadow-sm transition hover:border-teal-500/60 hover:bg-surface-container-low focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-teal-500/25'
      end

      def login_background
        div(class: 'pointer-events-none absolute inset-0 opacity-70') do
          div(class: 'absolute left-1/2 top-0 h-72 w-72 -translate-x-1/2 rounded-full bg-teal-200/40 blur-3xl dark:bg-teal-400/10')
          div(class: 'absolute bottom-0 right-0 h-96 w-96 rounded-full bg-blue-200/45 blur-3xl dark:bg-blue-500/10')
        end
      end
    end
  end
end
