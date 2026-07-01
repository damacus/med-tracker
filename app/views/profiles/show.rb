# frozen_string_literal: true

module Views
  module Profiles
    class Show < Views::Base
      include Phlex::Rails::Helpers::LinkTo
      include Phlex::Rails::Helpers::Routes
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::ButtonTo

      attr_reader :person, :account, :api_app_tokens, :new_api_app_token

      def initialize(person:, account:, api_app_tokens: nil, new_api_app_token: nil)
        super()
        @person = person
        @account = account
        @api_app_tokens = api_app_tokens || account.api_app_tokens.active.order(created_at: :desc).to_a
        @new_api_app_token = new_api_app_token
      end

      def view_template
        div(class: 'container mx-auto max-w-6xl px-4 py-8 pb-24 md:pb-8') do
          render_header
          render_profile_sections
        end
      end

      private

      def render_header
        div(class: header_classes, data: { testid: 'profile-hero' }) do
          div(class: 'flex flex-col gap-6 lg:flex-row lg:items-center lg:justify-between') do
            div(class: 'flex flex-col gap-5 sm:flex-row sm:items-center') do
              render Components::Shared::PersonAvatar.new(person: person, size: :xl)
              render_header_copy
            end
            div(class: 'w-full max-w-md space-y-3 lg:w-[25rem]', data: { testid: 'profile-identity-details' }) do
              render_header_stat(t('profiles.show.stats.profile_name'), person.name)
              render_header_stat(t('profiles.show.stats.sign_in_email'), account.email)
            end
          end
        end
      end

      def header_classes
        'mb-8 overflow-hidden rounded-shape-xl border border-outline-variant/70 bg-surface-container-low ' \
          'px-6 py-8 shadow-elevation-2 sm:px-8'
      end

      def render_header_copy
        div(class: 'max-w-2xl') do
          p(class: 'mb-3 text-[0.7rem] font-semibold uppercase tracking-[0.34em] text-on-surface-variant') do
            t('profiles.show.eyebrow')
          end
          h1(class: 'text-4xl font-semibold tracking-tight text-foreground sm:text-5xl') do
            t('profiles.show.title')
          end
          p(class: 'mt-3 max-w-xl text-sm leading-6 text-on-surface-variant sm:text-base') do
            t('profiles.show.description')
          end
        end
      end

      def render_profile_sections
        div(class: 'grid grid-cols-1 gap-6 xl:grid-cols-[minmax(0,1.35fr)_minmax(22rem,0.9fr)]') do
          render_left_column
          render_right_column
        end
      end

      def render_left_column
        div(class: 'space-y-6') do
          render ThemePickerCard.new
          render_avatar_card
          render_personal_info_card
          render TwoFactorCard.new(account: account)
        end
      end

      def render_right_column
        div(class: 'space-y-6') do
          render AccountSecurityCard.new(account: account)
          render_api_tokens_card
          render NotificationsCard.new(person: person)
          render ExperimentsCard.new(account: account)
          render DangerZoneCard.new
          render VersionInfo.new
        end
      end

      def render_api_tokens_card
        render ApiTokensCard.new(
          account: account,
          api_app_tokens: api_app_tokens,
          new_api_app_token: new_api_app_token
        )
      end

      def render_personal_info_card
        m3_card(
          class: 'overflow-hidden border border-border/70 shadow-elevation-2',
          data: { testid: 'profile-personal-info-card' }
        ) do
          render CardHeader.new do
            render(CardTitle.new { t('profiles.show.personal_information.title') })
            render(CardDescription.new { t('profiles.show.personal_information.description') })
          end
          render PersonalInfoContent.new(person: person, account: account)
        end
      end

      def render_header_stat(label, value)
        div(class: 'rounded-shape-xl border border-outline-variant/70 bg-surface-container px-4 py-4 shadow-elevation-1') do
          p(class: 'text-[0.68rem] font-semibold uppercase tracking-[0.22em] text-on-surface-variant') { label }
          p(class: 'mt-2 text-sm font-semibold leading-6 text-foreground break-all sm:text-base',
            data: { testid: "profile-identity-value-#{label.parameterize}" }) do
            value.presence || t('profiles.show.not_set')
          end
        end
      end

      def render_avatar_card
        m3_card(
          class: 'overflow-hidden border border-border/70 shadow-elevation-2',
          data: { testid: 'profile-avatar-card' }
        ) do
          m3_card_header do
            m3_card_title { t('profiles.avatar.title') }
            m3_card_description { t('profiles.avatar.description') }
          end
          m3_card_content(class: 'space-y-6') do
            render_avatar_identity
            render_avatar_upload_form
            render_gravatar_form
          end
        end
      end

      def render_avatar_identity
        div(class: 'flex flex-col gap-5 sm:flex-row sm:items-center') do
          render Components::Shared::PersonAvatar.new(person: person, size: :xl)
          div(class: 'space-y-1') do
            m3_text(variant: :title_medium, class: 'font-bold') { person.name }
            m3_text(variant: :body_medium, class: 'text-on-surface-variant') { t('profiles.avatar.supported_formats') }
          end
        end
      end

      def render_avatar_upload_form
        div(class: 'rounded-shape-xl border border-outline-variant/70 bg-surface-container p-4') do
          form_with(url: profile_path, method: :patch, multipart: true, class: 'space-y-4') do
            label(class: 'block text-sm font-bold text-foreground', for: 'person_avatar') do
              t('profiles.avatar.upload_label')
            end
            input(
              id: 'person_avatar',
              type: 'file',
              name: 'person[avatar]',
              accept: 'image/png,image/jpeg,image/webp',
              class: 'block w-full rounded-shape-md border border-outline-variant bg-surface px-3 py-2 text-sm text-on-surface'
            )
            m3_button(type: :submit, variant: :filled, size: :sm) { t('profiles.avatar.upload') }
          end
          render_avatar_remove_form if person.avatar.attached?
        end
      end

      def render_avatar_remove_form
        form_with(url: profile_avatar_path, method: :delete) do
          m3_button(type: :submit, variant: :outlined, size: :sm) { t('profiles.avatar.remove') }
        end
      end

      def render_gravatar_form
        div(class: 'rounded-shape-xl border border-outline-variant/70 bg-surface-container p-4') do
          form_with(url: profile_path, method: :patch, class: 'flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between') do
            div(class: 'flex items-start gap-3') do
              input(type: 'hidden', name: 'account[gravatar_enabled]', value: '0')
              input(
                id: 'account_gravatar_enabled',
                type: 'checkbox',
                name: 'account[gravatar_enabled]',
                value: '1',
                checked: account.gravatar_enabled?,
                class: 'mt-1 h-4 w-4 rounded border-outline text-primary focus:ring-primary'
              )
              label(class: 'space-y-1', for: 'account_gravatar_enabled') do
                span(class: 'block text-sm font-bold text-foreground') { t('profiles.avatar.gravatar_label') }
                span(class: 'block text-sm text-on-surface-variant') { t('profiles.avatar.gravatar_description') }
              end
            end
            m3_button(type: :submit, variant: :tonal, size: :sm) { t('profiles.avatar.save_gravatar') }
          end
        end
      end
    end
  end
end
