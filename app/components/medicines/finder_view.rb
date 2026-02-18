# frozen_string_literal: true

module Components
  module Medicines
    class FinderView < Components::Base
      def view_template
        div(
          data: { testid: 'medicine-finder', controller: 'medicine-search' },
          class: 'container mx-auto px-4 py-8 max-w-4xl'
        ) do
          render_header
          render_search_section
          render_results_section
        end
      end

      private

      def render_header
        div(class: 'mb-8') do
          Heading(level: 1, class: 'mb-2') { 'Medicine Finder' }
          Text(size: '2', class: 'text-slate-600') do
            plain 'Search the NHS Dictionary of Medicines and Devices (dm+d) by name or active ingredient.'
          end
        end
      end

      def render_search_section
        form(
          data: { action: 'submit->medicine-search#search', medicine_search_target: 'form' },
          class: 'mb-6'
        ) do
          div(class: 'flex gap-2') do
            input(
              type: 'text',
              id: 'medicine-search-input',
              name: 'q',
              placeholder: 'e.g. Aspirin, Ibuprofen, Nurofen...',
              autocomplete: 'off',
              data: { medicine_search_target: 'input' },
              class: 'flex-1 rounded-md border border-slate-300 px-4 py-2 text-sm ' \
                     'focus:border-primary focus:outline-none focus:ring-2 focus:ring-primary/20'
            )
            Button(
              type: :submit,
              size: :sm,
              data: { medicine_search_target: 'submitButton' }
            ) { 'Search' }
          end
        end
      end

      def render_results_section
        div(data: { medicine_search_target: 'results' }, class: 'space-y-2') do
          render_idle_state
        end
      end

      def render_idle_state
        div(data: { medicine_search_target: 'idle' }, class: 'text-center py-12 text-slate-500') do
          Text(size: '2') { 'Enter a medicine name above to search the NHS dm+d database.' }
        end
      end
    end
  end
end
