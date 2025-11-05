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
          div(class: 'space-y-4') do
            # Link to Users Management
            a(
              href: admin_users_path,
              class: 'block rounded-xl border border-slate-200 bg-white p-6 text-left shadow-sm transition hover:shadow-md'
            ) do
              h2(class: 'text-xl font-semibold text-slate-900') { 'User Management' }
              p(class: 'mt-2 text-slate-600') { 'Review and manage user accounts and access levels' }
            end

            # Link to Audit Trail
            a(
              href: admin_audit_logs_path,
              class: 'block rounded-xl border border-slate-200 bg-white p-6 text-left shadow-sm transition hover:shadow-md'
            ) do
              h2(class: 'text-xl font-semibold text-slate-900') { 'Audit Trail' }
              p(class: 'mt-2 text-slate-600') { 'View security audit logs of sensitive actions' }
            end
          end
        end
      end
    end
  end
end
