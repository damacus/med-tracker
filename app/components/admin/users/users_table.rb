# frozen_string_literal: true

module Components
  module Admin
    module Users
      # Renders the users table with sortable headers
      class UsersTable < Components::Base
        include Phlex::Rails::Helpers::FormWith

        attr_reader :users, :search_params, :current_user

        def initialize(users:, search_params: {}, current_user: nil)
          @users = users
          @search_params = search_params
          @current_user = current_user
          super()
        end

        def view_template
          render_mobile_cards
          div(class: 'hidden w-full overflow-x-auto md:block', data: { testid: 'admin-users-desktop-table' }) do
            render RubyUI::Table.new(class: 'min-w-[800px]') do
              render_table_header
              render_table_body
            end
          end
        end

        private

        def render_mobile_cards
          div(class: 'space-y-4 md:hidden', data: { testid: 'admin-users-mobile-list' }) do
            users.each do |user|
              row_class = user.active? && !user.soft_deleted? ? '' : 'opacity-60'
              m3_card(id: "mobile_user_#{user.id}",
                      class: 'rounded-[2rem] border border-outline-variant/40 bg-card p-5 ' \
                             "shadow-elevation-1 #{row_class}",
                      data: { user_id: user.id }) do
                div(class: 'space-y-4') do
                  div(class: 'flex items-start justify-between gap-3') do
                    div(class: 'min-w-0') do
                      m3_text(size: '2', weight: 'muted',
                              class: 'uppercase tracking-widest font-bold') { t('admin.users.form.name') }
                      m3_text(class: 'mt-1 break-words font-bold text-foreground') { user.name }
                    end
                    render_status_badge(user)
                  end

                  div(class: 'min-w-0') do
                    m3_text(size: '2', weight: 'muted',
                            class: 'uppercase tracking-widest font-bold') { t('admin.users.form.email_address') }
                    m3_text(class: 'mt-1 break-all font-medium text-foreground') { user.email_address }
                  end

                  dl(class: 'grid grid-cols-2 gap-3 border-t border-outline-variant/30 pt-4 text-sm') do
                    render_mobile_detail(t('admin.users.form.role'), user.role.to_s.capitalize)
                    div do
                      dt(class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant') do
                        t('admin.users.table.verification')
                      end
                      dd(class: 'mt-2') { render_verification_button(user) }
                    end
                  end

                  div(class: 'flex flex-wrap gap-2 border-t border-outline-variant/30 pt-4') do
                    render RubyUI::Link.new(
                      href: "/admin/users/#{user.id}/edit",
                      variant: :outlined,
                      size: :sm,
                      class: 'flex-1 rounded-xl',
                      data: { turbo_frame: '_top' }
                    ) { t('admin.users.user_row.edit') }
                    render_activation_button(user)
                  end
                end
              end
            end
          end
        end

        def render_mobile_detail(label, value)
          div do
            dt(class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant') { label }
            dd(class: 'mt-1 break-words font-semibold text-foreground') { value }
          end
        end

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

        def render_status_badge(user)
          if user.soft_deleted?
            render RubyUI::Badge.new(variant: :tonal) { t('admin.users.user_row.soft_deleted') }
          elsif user.active?
            render RubyUI::Badge.new(variant: :success) { t('admin.users.user_row.active') }
          else
            render RubyUI::Badge.new(variant: :destructive) { t('admin.users.user_row.inactive') }
          end
        end

        def render_activation_button(user)
          return if user.soft_deleted?

          if user.active?
            if user == current_user
              render M3::Button.new(variant: :destructive_outline, size: :sm, disabled: true,
                                    class: 'flex-1 rounded-xl') { t('admin.users.user_row.deactivate') }
            else
              render_deactivate_dialog(user)
            end
          else
            render_activate_button(user)
          end
        end

        def render_activate_button(user)
          form_with(
            url: "/admin/users/#{user.id}/activate",
            method: :post,
            class: 'flex-1'
          ) do
            m3_button(
              type: :submit,
              variant: :success_outline,
              size: :sm,
              class: 'w-full rounded-xl'
            ) { t('admin.users.user_row.activate') }
          end
        end

        def render_verification_button(user)
          return render_soft_deleted_button if user.soft_deleted?
          return render_verified_button if user.person&.account&.verified?

          form_with(
            url: "/admin/users/#{user.id}/verify",
            method: :post,
            class: 'inline-block'
          ) do
            m3_button(
              type: :submit,
              variant: :outlined,
              size: :sm
            ) { t('admin.users.user_row.verify') }
          end
        end

        def render_verified_button
          m3_button(
            type: :button,
            variant: :outlined,
            size: :sm,
            disabled: true
          ) { t('admin.users.user_row.verified') }
        end

        def render_soft_deleted_button
          m3_button(
            type: :button,
            variant: :outlined,
            size: :sm,
            disabled: true
          ) { t('admin.users.user_row.soft_deleted') }
        end

        def render_deactivate_dialog(user)
          render RubyUI::AlertDialog.new do
            render RubyUI::AlertDialogTrigger.new do
              m3_button(variant: :destructive_outline, size: :sm, class: 'w-full rounded-xl') do
                t('admin.users.user_row.deactivate')
              end
            end
            render RubyUI::AlertDialogContent.new do
              render RubyUI::AlertDialogHeader.new do
                render(RubyUI::AlertDialogTitle.new { t('admin.users.user_row.deactivate_dialog.title') })
                render RubyUI::AlertDialogDescription.new do
                  t('admin.users.user_row.deactivate_dialog.confirm', name: user.name)
                end
              end
              render RubyUI::AlertDialogFooter.new do
                render(RubyUI::AlertDialogCancel.new { t('admin.users.user_row.deactivate_dialog.cancel') })
                form_with(url: "/admin/users/#{user.id}", method: :delete, class: 'inline') do
                  m3_button(variant: :destructive, type: :submit) { t('admin.users.user_row.deactivate_dialog.submit') }
                end
              end
            end
          end
        end
      end
    end
  end
end
