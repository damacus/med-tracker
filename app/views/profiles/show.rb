# frozen_string_literal: true

module Views
  module Profiles
    class Show < Views::Base
      include Phlex::Rails::Helpers::LinkTo
      include Phlex::Rails::Helpers::Routes
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::ButtonTo

      attr_reader :person, :account, :user

      def initialize(person:, account:, user:)
        super()
        @person = person
        @account = account
        @user = user
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
          div(class: 'relative flex flex-col gap-6 lg:flex-row lg:items-center lg:justify-between') do
            render_header_copy
            div(class: 'grid gap-3 sm:grid-cols-2 lg:w-[24rem]') do
              render_header_stat(t('profiles.show.stats.profile_name'), person.name)
              render_header_stat(t('profiles.show.stats.sign_in_email'), account.email)
            end
          end
        end
      end

      def header_classes
        'relative mb-8 overflow-hidden border border-border/70 bg-card px-6 py-8 shadow-elevation-2 sm:px-8'
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
          render_personal_info_card
          render TwoFactorCard.new(account: account)
        end
      end

      def render_right_column
        div(class: 'space-y-6') do
          render AccountSecurityCard.new(account: account)
          render NotificationsCard.new(person: person)
          render ExperimentsCard.new(user: user)
          render DangerZoneCard.new
          render VersionInfo.new
        end
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
        div(class: 'rounded-shape-xl border border-border/70 bg-popover px-4 py-4 shadow-elevation-1') do
          p(class: 'text-[0.68rem] font-semibold uppercase tracking-[0.22em] text-on-surface-variant') { label }
          p(class: 'mt-2 truncate text-sm font-semibold text-foreground sm:text-base') { value.presence || t('profiles.show.not_set') }
        end
      end
    end
  end
end
