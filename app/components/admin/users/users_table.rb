# frozen_string_literal: true

module Components
  module Admin
    module Users
      # Renders the users table with sortable headers
      class UsersTable < Components::Base
        attr_reader :users, :search_params, :current_user

        def initialize(users:, search_params: {}, current_user: nil)
          @users = users
          @search_params = search_params
          @current_user = current_user
          super()
        end

        def view_template
          div(class: 'w-full overflow-x-auto') do
            render RubyUI::Table.new(class: 'min-w-[800px]') do
              render_table_header
              render_table_body
            end
          end
        end

        private

        def render_table_header
          render RubyUI::TableHeader.new do
            render RubyUI::TableRow.new do
              render(RubyUI::TableHead.new { render_sortable_header(t('admin.users.form.name'), 'name') })
              render(RubyUI::TableHead.new { render_sortable_header(t('admin.users.form.email_address'), 'email') })
              render(RubyUI::TableHead.new { render_sortable_header(t('admin.users.form.role'), 'role') })
              render(RubyUI::TableHead.new { t('admin.users.table.activation') })
              render(RubyUI::TableHead.new { t('admin.users.table.verification') })
              render RubyUI::TableHead.new(class: 'text-center') { t('admin.users.table.actions') }
            end
          end
        end

        def render_sortable_header(label, column)
          current_sort = search_params[:sort]
          current_direction = search_params[:direction] || 'asc'
          is_active = current_sort == column

          new_direction = is_active && current_direction == 'asc' ? 'desc' : 'asc'
          sort_params = search_params.to_h.merge(sort: column, direction: new_direction)

          Link(
            href: "/admin/users?#{sort_params.to_query}",
            variant: :link,
            class: sortable_header_class(is_active),
            data: { turbo_frame: 'admin-users-frame' }
          ) do
            span { label }
            render_sort_indicator(is_active, current_direction)
          end
        end

        def sortable_header_class(is_active)
          base = 'inline-flex items-center gap-1 hover:text-foreground cursor-pointer'
          is_active ? "#{base} text-foreground font-semibold" : "#{base} text-on-surface-variant"
        end

        def render_sort_indicator(is_active, direction)
          return unless is_active

          span(class: 'text-xs') do
            direction == 'asc' ? '↑' : '↓'
          end
        end

        def render_table_body
          render RubyUI::TableBody.new do
            users.each do |user|
              render Components::Admin::Users::UserRow.new(user: user, current_user: current_user)
            end
          end
        end
      end
    end
  end
end