# frozen_string_literal: true

module Components
  module Medicines
    class FinderView < Components::Base
      def view_template
        div(data: { testid: 'medicine-finder' }, class: 'container mx-auto px-4 py-8 max-w-4xl') do
          render_header
          render_search_section
          render_coming_soon
        end
      end

      private

      def render_header
        div(class: 'mb-8') do
          Heading(level: 1, class: 'mb-2') { 'Medicine Finder' }
        end
      end

      def render_search_section
        div(class: 'mb-8') do
          div(class: 'flex gap-2 mb-4') do
            input(
              type: 'text',
              id: 'medicine-search-input',
              placeholder: 'Search for medicines...',
              class: 'flex-1 rounded-md border border-slate-300 px-4 py-2 text-sm ' \
                     'focus:border-primary focus:outline-none focus:ring-2 focus:ring-primary/20'
            )
            Button(size: :sm) { 'Search' }
          end

          Text(size: '2', class: 'text-slate-600') do
            plain 'Search for medicines by name or active ingredient. ' \
                  'Results will be provided by an online medicine database.'
          end
        end
      end

      def render_coming_soon
        render RubyUI::Card.new(class: 'bg-slate-50') do
          render RubyUI::CardContent.new(class: 'py-8 text-center') do
            Text(class: 'text-slate-600') do
              plain t('medicines.finder.coming_soon')
            end
          end
        end
      end
    end
  end
end
