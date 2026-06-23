# frozen_string_literal: true

module Components
  module GlobalSearch
    class ShowView < Components::Base
      attr_reader :query, :results

      def initialize(query:, results:)
        @query = query
        @results = results
        super()
      end

      def view_template
        div(class: 'container mx-auto max-w-4xl px-4 py-10 space-y-8') do
          header(class: 'space-y-2') do
            m3_heading(variant: :display_small, level: 1) { t('global_search.page_title') }
            m3_text(class: 'text-on-surface-variant') { t('global_search.page_subtitle') }
          end
          render_form
          render_results
        end
      end

      private

      def render_form
        form(action: household_search_path, method: :get, class: 'flex flex-col gap-3 sm:flex-row') do
          label(class: 'sr-only', for: 'search_q') { t('global_search.input_label') }
          input(
            id: 'search_q',
            name: 'q',
            type: 'search',
            value: query,
            class: 'min-h-[44px] flex-1 rounded-shape-sm border border-outline-variant bg-surface-container-low px-4',
            placeholder: t('global_search.placeholder')
          )
          m3_button(type: :submit) { t('global_search.page_submit') }
        end
      end

      def household_search_path
        return root_path unless Current.household

        search_path(household_slug: Current.household.slug)
      end

      def render_results
        if results.empty?
          m3_text(class: 'text-on-surface-variant') { t('global_search.no_results') }
          return
        end

        div(class: 'space-y-2') do
          results.each do |result|
            link_to result.path,
                    class: 'block rounded-md border border-outline-variant bg-surface-container-low p-4 ' \
                           'no-underline hover:border-primary focus-visible:outline-none focus-visible:ring-2 ' \
                           'focus-visible:ring-primary' do
              p(class: 'font-bold text-foreground') { result.title }
              p(class: 'text-sm text-on-surface-variant') { result.subtitle }
            end
          end
        end
      end
    end
  end
end
