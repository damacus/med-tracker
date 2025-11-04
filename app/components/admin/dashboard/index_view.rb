# frozen_string_literal: true

module Components
  module Admin
    module Dashboard
      # Admin dashboard placeholder view
      class IndexView < Components::Base
        def view_template
          div(data: { testid: 'admin-dashboard' }, class: 'space-y-8 p-8') do
            render_header
            render_placeholder
          end
        end

        private

        def render_header
          header(class: 'space-y-2') do
            h1(class: 'text-3xl font-semibold text-slate-900') { 'Admin Dashboard' }
            p(class: 'text-slate-600') { 'Administrative tools and system overview' }
          end
        end

        def render_placeholder
          div(class: 'rounded-xl border border-slate-200 bg-white p-12 text-center shadow-sm') do
            div(class: 'space-y-4') do
              h2(class: 'text-2xl font-semibold text-slate-700') { 'Coming Soon' }
              p(class: 'text-slate-600') do
                'This dashboard is under construction. Check back soon for administrative tools and insights.'
              end
            end
          end
        end
      end
    end
  end
end
