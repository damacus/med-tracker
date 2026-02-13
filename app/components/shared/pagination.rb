# frozen_string_literal: true

module Components
  module Shared
    class Pagination < Components::Base
      attr_reader :pagy_obj, :base_url, :extra_params

      def initialize(pagy:, base_url:, extra_params: {})
        @pagy_obj = pagy
        @base_url = base_url
        @extra_params = extra_params
        super()
      end

      def view_template
        div(class: 'flex items-center justify-between border-t border-slate-200 ' \
                   'bg-white px-4 py-3 sm:px-6') do
          if pagy_obj.pages > 1
            div(class: 'flex flex-1 justify-between sm:hidden') do
              render_mobile_pagination
            end
          end
          div(class: 'hidden sm:flex sm:flex-1 sm:items-center sm:justify-between') do
            render_pagination_info
            render_pagination_nav if pagy_obj.pages > 1
          end
        end
      end

      private

      def render_mobile_pagination
        if pagy_obj.previous
          Link(href: page_url(pagy_obj.previous), variant: :link,
               class: mobile_nav_class) { 'Previous' }
        else
          span(class: "#{mobile_nav_class} opacity-50 cursor-not-allowed") { 'Previous' }
        end

        if pagy_obj.next
          Link(href: page_url(pagy_obj.next), variant: :link,
               class: mobile_nav_class) { 'Next' }
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
        nav(class: 'isolate inline-flex -space-x-px rounded-md shadow-sm',
            aria: { label: 'Pagination' }) do
          render_prev_button
          render_page_numbers
          render_next_button
        end
      end

      def render_prev_button
        render_nav_button(page: pagy_obj.previous, label: 'Previous', icon: '‹', rounding: 'rounded-l-md')
      end

      def render_next_button
        render_nav_button(page: pagy_obj.next, label: 'Next', icon: '›', rounding: 'rounded-r-md')
      end

      def render_nav_button(page:, label:, icon:, rounding:)
        if page
          Link(href: page_url(page), variant: :link,
               class: nav_button_class(rounding)) do
            span(class: 'sr-only') { label }
            plain icon
          end
        else
          span(class: "#{nav_button_class(rounding)} opacity-50 cursor-not-allowed") do
            span(class: 'sr-only') { label }
            plain icon
          end
        end
      end

      def render_page_numbers
        pagy_obj.series.each do |item|
          case item
          when Integer
            Link(href: page_url(item), variant: :link,
                 class: page_number_class(false)) { item.to_s }
          when String
            span(class: page_number_class(true)) { item }
          when :gap
            span(class: gap_class) { '…' }
          end
        end
      end

      def page_url(page)
        params = extra_params.to_h.merge(page: page)
        "#{base_url}?#{params.to_query}"
      end

      def nav_button_class(extra = '')
        'relative inline-flex items-center px-2 py-2 text-slate-400 ' \
          'ring-1 ring-inset ring-slate-300 hover:bg-slate-50 ' \
          "focus:z-20 focus:outline-offset-0 #{extra}"
      end

      def page_number_class(current)
        base = 'relative inline-flex items-center px-4 py-2 text-sm ' \
               'font-semibold ring-1 ring-inset ring-slate-300 ' \
               'focus:z-20 focus:outline-offset-0'
        if current
          "#{base} z-10 bg-primary text-white"
        else
          "#{base} text-slate-900 hover:bg-slate-50"
        end
      end

      def gap_class
        'relative inline-flex items-center px-4 py-2 text-sm font-semibold ' \
          'text-slate-700 ring-1 ring-inset ring-slate-300'
      end
    end
  end
end
