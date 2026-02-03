# frozen_string_literal: true

module Components
  module Admin
    module Users
      # Renders pagination controls for the users list
      class Pagination < Components::Base
        attr_reader :pagy_obj, :search_params

        def initialize(pagy:, search_params: {})
          @pagy_obj = pagy
          @search_params = search_params
          super()
        end

        def view_template
          div(class: 'flex items-center justify-between border-t border-slate-200 bg-white px-4 py-3 sm:px-6') do
            div(class: 'flex flex-1 justify-between sm:hidden') do
              render_mobile_pagination
            end
            div(class: 'hidden sm:flex sm:flex-1 sm:items-center sm:justify-between') do
              render_pagination_info
              render_pagination_nav
            end
          end
        end

        private

        def render_mobile_pagination
          if pagy_obj.previous
            Link(href: page_url(pagy_obj.previous), variant: :link, class: mobile_nav_class) { 'Previous' }
          else
            span(class: "#{mobile_nav_class} opacity-50 cursor-not-allowed") { 'Previous' }
          end

          if pagy_obj.next
            Link(href: page_url(pagy_obj.next), variant: :link, class: mobile_nav_class) { 'Next' }
          else
            span(class: "#{mobile_nav_class} opacity-50 cursor-not-allowed") { 'Next' }
          end
        end

        def mobile_nav_class
          'relative inline-flex items-center rounded-md border border-slate-300 bg-white ' \
            'px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50'
        end

        def render_pagination_info
          div(data: { testid: 'pagination-info' }) do
            Text(size: '2', class: 'text-slate-700') do
              plain 'Showing '
              span(class: 'font-medium') { pagy_obj.from.to_s }
              plain ' to '
              span(class: 'font-medium') { pagy_obj.to.to_s }
              plain ' of '
              span(class: 'font-medium') { pagy_obj.count.to_s }
              plain ' results'
            end
          end
        end

        def render_pagination_nav
          return if pagy_obj.pages <= 1

          nav(class: 'isolate inline-flex -space-x-px rounded-md shadow-sm', aria: { label: 'Pagination' }) do
            render_prev_button
            render_page_numbers
            render_next_button
          end
        end

        def render_prev_button
          if pagy_obj.previous
            Link(href: page_url(pagy_obj.previous), variant: :link, class: nav_button_class('rounded-l-md')) do
              span(class: 'sr-only') { 'Previous' }
              plain '‹'
            end
          else
            span(class: "#{nav_button_class('rounded-l-md')} opacity-50 cursor-not-allowed") do
              span(class: 'sr-only') { 'Previous' }
              plain '‹'
            end
          end
        end

        def render_next_button
          if pagy_obj.next
            Link(href: page_url(pagy_obj.next), variant: :link, class: nav_button_class('rounded-r-md')) do
              span(class: 'sr-only') { 'Next' }
              plain '›'
            end
          else
            span(class: "#{nav_button_class('rounded-r-md')} opacity-50 cursor-not-allowed") do
              span(class: 'sr-only') { 'Next' }
              plain '›'
            end
          end
        end

        def render_page_numbers
          pagy_obj.series.each do |item|
            case item
            when Integer
              Link(href: page_url(item), variant: :link, class: page_number_class(false)) { item.to_s }
            when String
              span(class: page_number_class(true)) { item }
            when :gap
              span(class: gap_class) { '…' }
            end
          end
        end

        def page_url(page)
          params = search_params.to_h.merge(page: page)
          "/admin/users?#{params.to_query}"
        end

        def nav_button_class(extra = '')
          'relative inline-flex items-center px-2 py-2 text-slate-400 ring-1 ring-inset ' \
            "ring-slate-300 hover:bg-slate-50 focus:z-20 focus:outline-offset-0 #{extra}"
        end

        def page_number_class(current)
          base = 'relative inline-flex items-center px-4 py-2 text-sm font-semibold ' \
                 'ring-1 ring-inset ring-slate-300 focus:z-20 focus:outline-offset-0'
          current ? "#{base} z-10 bg-primary text-white" : "#{base} text-slate-900 hover:bg-slate-50"
        end

        def gap_class
          'relative inline-flex items-center px-4 py-2 text-sm font-semibold ' \
            'text-slate-700 ring-1 ring-inset ring-slate-300'
        end
      end
    end
  end
end
