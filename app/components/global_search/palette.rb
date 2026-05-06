# frozen_string_literal: true

module Components
  module GlobalSearch
    class Palette < Components::Base
      def view_template
        dialog(
          id: 'global_search_dialog',
          class: 'w-[min(720px,calc(100vw-2rem))] max-h-[min(680px,calc(100vh-2rem))] p-0 ' \
                 'overflow-hidden rounded-lg border border-outline-variant bg-surface text-on-surface ' \
                 'shadow-elevation-4 backdrop:bg-black/40',
          aria: { label: t('global_search.dialog_label') },
          data: dialog_data
        ) do
          render_search_form
          render_status
          div(
            id: 'global_search_results',
            class: 'max-h-[520px] overflow-y-auto p-3',
            data: { global_search_target: 'results' }
          )
        end
      end

      private

      def dialog_data
        {
          global_search_target: 'dialog',
          action: 'cancel->global-search#cancel close->global-search#closed',
          search_url: search_path(format: :json),
          translations: translations.to_json
        }
      end

      def render_search_form
        form(action: search_path, method: :get, class: 'border-b border-outline-variant p-3',
             data: { action: 'submit->global-search#submit' }) do
          label(class: 'sr-only', for: 'global_search_query') { t('global_search.input_label') }
          div(class: 'flex items-center gap-3 rounded-md bg-surface-container-low px-3 py-2') do
            render Icons::Search.new(size: 20, class: 'text-on-surface-variant')
            input(
              id: 'global_search_query',
              name: 'q',
              type: 'search',
              autocomplete: 'off',
              placeholder: t('global_search.placeholder'),
              class: 'min-h-[44px] flex-1 bg-transparent text-base text-foreground outline-none ' \
                     'placeholder:text-on-surface-variant',
              aria: { controls: 'global_search_results' },
              data: {
                global_search_target: 'input',
                action: 'input->global-search#search keydown->global-search#handleKeydown'
              }
            )
            button(
              type: 'button',
              class: 'flex h-10 w-10 items-center justify-center rounded-full text-on-surface-variant ' \
                     'hover:bg-surface-container-high focus-visible:outline-none focus-visible:ring-2 ' \
                     'focus-visible:ring-primary',
              aria: { label: t('global_search.close') },
              data: { action: 'global-search#close' }
            ) do
              render Icons::X.new(size: 20)
            end
          end
        end
      end

      def render_status
        div(
          class: 'px-4 py-2 text-xs font-bold uppercase tracking-widest text-on-surface-variant',
          aria: { live: 'polite' },
          data: { global_search_target: 'status' }
        )
      end

      def translations
        {
          loading: t('global_search.loading'),
          no_results: t('global_search.no_results'),
          result_one: t('global_search.results.one'),
          result_other: t('global_search.results.other'),
          type_labels: {
            command: t('global_search.types.command'),
            person: t('global_search.types.person'),
            medication: t('global_search.types.medication'),
            person_medication: t('global_search.types.person_medication'),
            schedule: t('global_search.types.schedule'),
            location: t('global_search.types.location')
          }
        }
      end
    end
  end
end
