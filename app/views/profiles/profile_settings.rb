# frozen_string_literal: true

module Views
  module Profiles
    class ProfileSettings < Views::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::Routes

      def initialize(person:, account:)
        super()
        @person = person
        @account = account
      end

      def view_template
        div(class: 'space-y-3') do
          render_avatar_sheet
          render_time_zone_dialog
          render_appearance_sheet
        end
      end

      private

      def render_avatar_sheet
        render_setting_sheet(
          title: t('profiles.avatar.title'),
          description: t('profiles.avatar.description'),
          testid: 'profile-avatar-sheet'
        ) do
          render_sheet_error('profile-avatar-errors')
          render_avatar_identity
          render_avatar_upload_form
          render_gravatar_form
        end
      end

      def render_time_zone_dialog
        title = t('profiles.time_zone.title')
        description = t('profiles.time_zone.description')
        div(data: { testid: 'profile-time-zone-dialog' }) do
          render Dialog.new do
            render DialogTrigger.new(class: 'block w-full') do
              render_setting_button(title:, description:)
            end
            render DialogContent.new(
              size: :sm,
              class: 'max-h-[calc(100vh-2rem)] w-[calc(100vw-2rem)] rounded-shape-xl p-0'
            ) do
              render_dialog_header(title:, description:)
              render DialogMiddle.new(class: 'px-5 pb-5 pt-4 sm:px-6 sm:pb-6') do
                render_sheet_error('profile-time-zone-errors')
                render TimeZoneForm.new(account: @account)
              end
            end
          end
        end
      end

      def render_appearance_sheet
        render_setting_sheet(
          title: t('profiles.appearance.title'),
          description: t('profiles.appearance.description'),
          testid: 'profile-appearance-sheet',
          size: :xl
        ) do
          render ThemePickerCard.new
        end
      end

      def render_setting_sheet(title:, description:, testid:, size: :md, &content)
        render Sheet.new(data: { testid: }) do
          render_setting_trigger(title:, description:)
          render SheetContent.new(side: :right, size:, class: 'w-[calc(100vw-1rem)] p-5 sm:w-[calc(100vw-2rem)] sm:p-6') do
            render_sheet_header(title:, description:)
            render SheetMiddle.new(class: 'mt-6 flex-1 space-y-5', &content)
          end
        end
      end

      def render_setting_trigger(title:, description:)
        render SheetTrigger.new do
          render_setting_button(title:, description:, expanded: false)
        end
      end

      def render_setting_button(title:, description:, expanded: nil)
        button(type: 'button', class: setting_trigger_classes, aria: { expanded: }) do
          div(class: 'min-w-0 text-left') do
            p(class: 'font-semibold text-foreground') { title }
            p(class: 'mt-1 text-sm leading-5 text-on-surface-variant') { description }
          end
          render Components::Icons::ChevronRight.new(size: 20, aria_hidden: 'true')
        end
      end

      def render_sheet_header(title:, description:)
        render SheetHeader.new do
          render(SheetTitle.new { title })
          render(SheetDescription.new { description })
        end
      end

      def render_dialog_header(title:, description:)
        render DialogHeader.new(class: 'px-5 pb-4 pt-6 text-left sm:px-6') do
          render(DialogTitle.new(class: 'text-xl') { title })
          render(DialogDescription.new(class: 'text-sm leading-6') { description })
        end
      end

      def render_avatar_identity
        div(class: 'flex items-center gap-4') do
          render Components::Shared::PersonAvatar.new(person: @person, size: :xl)
          div do
            p(class: 'font-semibold text-foreground') { @person.name }
            p(class: 'text-sm text-on-surface-variant') { t('profiles.avatar.supported_formats') }
          end
        end
      end

      def render_avatar_upload_form
        form_with(url: profile_path, method: :patch, multipart: true, class: 'space-y-4') do
          input(type: 'hidden', name: 'section', value: 'profile')
          input(type: 'hidden', name: 'profile_setting', value: 'avatar')
          label(class: 'block text-sm font-bold text-foreground', for: 'person_avatar') do
            t('profiles.avatar.upload_label')
          end
          input(
            id: 'person_avatar', type: 'file', name: 'person[avatar]', accept: 'image/png,image/jpeg,image/webp',
            class: 'block w-full rounded-shape-md border border-outline-variant bg-surface px-3 py-3 text-sm text-on-surface'
          )
          m3_button(type: :submit, variant: :filled, size: :sm) { t('profiles.avatar.upload') }
        end
        render_avatar_remove_form if @person.avatar.attached?
      end

      def render_avatar_remove_form
        form_with(url: profile_avatar_path, method: :delete) do
          input(type: 'hidden', name: 'section', value: 'profile')
          m3_button(type: :submit, variant: :outlined, size: :sm) { t('profiles.avatar.remove') }
        end
      end

      def render_gravatar_form
        form_with(url: profile_path, method: :patch, class: 'space-y-4 border-t border-border pt-5') do
          input(type: 'hidden', name: 'section', value: 'profile')
          input(type: 'hidden', name: 'profile_setting', value: 'avatar')
          div(class: 'flex items-center justify-between gap-4') do
            div(class: 'min-w-0') do
              p(class: 'text-sm font-bold text-foreground') { t('profiles.avatar.gravatar_label') }
              p(class: 'mt-1 text-sm leading-5 text-on-surface-variant') { t('profiles.avatar.gravatar_description') }
            end
            render_binary_toggle(
              name: 'account[gravatar_enabled]',
              value: @account.gravatar_enabled? ? '1' : '0',
              label: t('profiles.avatar.gravatar_label')
            )
          end
          render_sheet_actions(t('profiles.avatar.save_gravatar'))
        end
      end

      def render_binary_toggle(name:, value:, label:)
        render ToggleGroup.new(type: :single, name:, value:, variant: :outline, size: :sm, aria: { label: }) do |group|
          group.toggle_group_item(value: '1', class: 'min-h-11') { t('profiles.common.on') }
          group.toggle_group_item(value: '0', class: 'min-h-11') { t('profiles.common.off') }
        end
      end

      def render_sheet_actions(save_label)
        render SheetFooter.new(class: 'mt-6 flex flex-row justify-end gap-3') do
          m3_button(type: 'button', variant: :outlined, data: { action: 'click->ruby-ui--sheet-content#close' }) do
            t('ruby_ui.common.close')
          end
          m3_button(type: :submit, variant: :filled) { save_label }
        end
      end

      def render_sheet_error(id)
        div(id:, aria: { live: 'polite' })
      end

      def setting_trigger_classes
        'flex min-h-16 w-full items-center justify-between gap-4 rounded-shape-lg border border-border/70 ' \
          'bg-popover px-4 py-3 text-left shadow-elevation-1 transition-colors hover:border-primary/30 ' \
          'focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2'
      end
    end
  end
end
