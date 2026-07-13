# frozen_string_literal: true

module Components
  module Platform
    module Users
      class IndexView < Components::Base
        include Phlex::Rails::Helpers::FormWith

        attr_reader :users, :current_user, :access_summary

        def initialize(users:, current_user:, access_summary:)
          @users = users
          @current_user = current_user
          @access_summary = access_summary
          super()
        end

        def view_template
          div(data: { testid: 'platform-users' },
              class: 'container mx-auto px-4 py-8 pb-24 md:pb-8 max-w-6xl space-y-8') do
            render_header
            render_table
          end
        end

        private

        def render_header
          div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 mb-12') do
            div do
              m3_text(size: '2', weight: 'muted', class: 'uppercase tracking-widest mb-1 block font-bold') do
                Time.current.strftime('%A, %b %d')
              end
              m3_heading(level: 1, size: '8', class: 'font-extrabold tracking-tight') do
                t('platform.users.title')
              end
              m3_text(weight: 'muted', class: 'mt-2 block') { t('platform.users.subtitle') }
            end
            render RubyUI::Link.new(href: platform_settings_path, variant: :outlined, size: :lg) do
              t('platform.users.settings')
            end
          end
        end

        def render_table
          div(class: 'rounded-[2rem] border border-border bg-card shadow-sm overflow-x-auto p-4') do
            render RubyUI::Table.new(class: 'min-w-[900px]') do
              render_table_header
              render_table_body
            end
          end
        end

        def render_table_header
          render RubyUI::TableHeader.new do
            render RubyUI::TableRow.new do
              render(RubyUI::TableHead.new { t('admin.users.form.name') })
              render(RubyUI::TableHead.new { t('admin.users.form.email_address') })
              render(RubyUI::TableHead.new { t('platform.users.household_role') })
              render(RubyUI::TableHead.new { t('platform.users.system_access') })
              render RubyUI::TableHead.new(class: 'text-right') { t('admin.users.table.actions') }
            end
          end
        end

        def render_table_body
          render RubyUI::TableBody.new do
            users.each { |user| render_user_row(user) }
          end
        end

        def render_user_row(user)
          render RubyUI::TableRow.new do
            render(RubyUI::TableCell.new(class: 'font-medium') { user.name })
            render(RubyUI::TableCell.new { user.email_address })
            render(RubyUI::TableCell.new { household_role_for(user) })
            render(RubyUI::TableCell.new { render_platform_access_badge(user) })
            render RubyUI::TableCell.new(class: 'text-right') { render_user_actions(user) }
          end
        end

        def render_platform_access_badge(user)
          if platform_admin?(user)
            render RubyUI::Badge.new(variant: :destructive) { t('platform.users.system_administrator') }
          else
            render RubyUI::Badge.new(variant: :tonal) { t('platform.users.household_user') }
          end
        end

        def render_platform_access_form(user)
          form_with(url: platform_user_path(user), method: :patch, class: 'inline-block') do
            input(
              type: 'hidden',
              name: 'platform_user[system_administrator]',
              value: platform_admin?(user) ? '0' : '1'
            )
            m3_button(type: :submit, variant: platform_admin?(user) ? :destructive_outline : :filled, size: :sm) do
              platform_admin?(user) ? t('platform.users.remove_system_access') : t('platform.users.grant_system_access')
            end
          end
        end

        def render_user_actions(user)
          div(class: 'flex flex-wrap justify-end gap-2') do
            render_platform_access_form(user)
            access_summary.promotable_memberships_for(user.person&.account_id).each do |membership|
              render_owner_promotion_form(membership)
            end
          end
        end

        def render_owner_promotion_form(membership)
          form_with(
            url: platform_promote_household_owner_path(membership.household, membership),
            method: :patch,
            class: 'inline-block'
          ) do
            render RubyUI::Button.new(variant: :outline, type: :submit, class: 'min-h-[44px]') do
              t('platform.users.promote_to_owner')
            end
          end
        end

        def household_role_for(user)
          role = access_summary.membership_role_for(user.person&.account_id)
          role&.titleize || t('admin.users.form.no_membership', default: 'No membership')
        end

        def platform_admin?(user)
          access_summary.platform_admin?(user.person&.account_id)
        end
      end
    end
  end
end
