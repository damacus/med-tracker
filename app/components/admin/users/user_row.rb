# frozen_string_literal: true

module Components
  module Admin
    module Users
      # Renders a single user row in the users table
      class UserRow < Components::Base
        include Phlex::Rails::Helpers::FormWith

        attr_reader :user, :current_user, :household

        def initialize(user:, current_user: nil, household: nil, membership_role: nil)
          @user = user
          @current_user = current_user
          @household = household || Current.household
          @membership_role = membership_role
          super()
        end

        def view_template
          row_class = user.active? && !user.soft_deleted? ? '' : 'opacity-60'
          render RubyUI::TableRow.new(id: "user_#{user.id}", class: row_class, data: { user_id: user.id }) do
            render RubyUI::TableCell.new(class: 'font-medium') { user.name }
            render(RubyUI::TableCell.new { user.email_address })
            render RubyUI::TableCell.new(class: 'capitalize') { membership_role }
            render(RubyUI::TableCell.new { render_status_badge })
            render(RubyUI::TableCell.new { render_verification_button })
            render RubyUI::TableCell.new(class: 'text-center') do
              div(class: 'flex gap-2 justify-center') do
                render RubyUI::Link.new(
                  href: edit_admin_user_path(user),
                  variant: :outlined,
                  size: :sm,
                  data: { turbo_frame: '_top' }
                ) { t('admin.users.user_row.edit') }
                render_activation_button
              end
            end
          end
        end

        private

        def membership_role
          return @membership_role if @membership_role

          lookup_membership_role
        end

        def lookup_membership_role
          account_id = user.person&.account_id
          return no_membership_label unless household && account_id

          membership = household.household_memberships.active.find_by(account_id: account_id)
          membership&.role&.titleize || no_membership_label
        end

        def no_membership_label
          t('admin.users.form.no_membership', default: 'No membership')
        end

        def render_status_badge
          if user.soft_deleted?
            render RubyUI::Badge.new(variant: :tonal) { t('admin.users.user_row.soft_deleted') }
          elsif user.active?
            render RubyUI::Badge.new(variant: :success) { t('admin.users.user_row.active') }
          else
            render RubyUI::Badge.new(variant: :destructive) { t('admin.users.user_row.inactive') }
          end
        end

        def render_activation_button
          return if user.soft_deleted?

          is_current_user = user == current_user

          if user.active?
            if is_current_user
              render M3::Button.new(variant: :destructive_outline, size: :sm, disabled: true) { t('admin.users.user_row.deactivate') }
            else
              render_deactivate_dialog
            end
          else
            render_activate_button
          end
        end

        def render_activate_button
          form_with(
            url: activate_admin_user_path(user),
            method: :post,
            class: 'inline-block'
          ) do
            m3_button(
              type: :submit,
              variant: :success_outline,
              size: :sm
            ) { t('admin.users.user_row.activate') }
          end
        end

        def render_verification_button
          return render_soft_deleted_button if user.soft_deleted?
          return render_verified_button if user.person&.account&.verified?

          form_with(
            url: verify_admin_user_path(user),
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

        def render_deactivate_dialog
          render RubyUI::AlertDialog.new do
            render RubyUI::AlertDialogTrigger.new do
              m3_button(variant: :destructive_outline, size: :sm) do
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
                form_with(url: admin_user_path(user), method: :delete, class: 'inline') do
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
