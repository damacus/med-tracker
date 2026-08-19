# frozen_string_literal: true

module Views
  module Profiles
    class Show < Views::Base
      attr_reader :presenter

      def initialize(presenter:)
        super()
        @presenter = presenter
      end

      def view_template
        div(class: 'container mx-auto max-w-6xl px-4 py-6 pb-24 sm:py-8 md:pb-8') do
          render_header
          render_sections
        end
      end

      private

      def render_header
        div(
          class: 'mb-6 rounded-shape-xl border border-outline-variant/70 bg-surface-container-low p-5 shadow-elevation-1 sm:p-6',
          data: { testid: 'profile-hero' }
        ) do
          div(class: 'flex items-center gap-4') do
            render Components::Shared::PersonAvatar.new(person: presenter.person, size: :lg)
            div(class: 'min-w-0') do
              p(class: 'text-[0.7rem] font-semibold uppercase tracking-[0.28em] text-on-surface-variant') do
                t('profiles.show.eyebrow')
              end
              h1(class: 'mt-1 text-3xl font-semibold tracking-tight text-foreground sm:text-4xl') do
                t('profiles.show.title')
              end
              p(class: 'mt-1 break-all text-sm text-on-surface-variant') { presenter.account.email }
            end
          end
          p(class: 'mt-4 max-w-2xl text-sm leading-6 text-on-surface-variant') do
            t('profiles.show.description')
          end
        end
      end

      def render_sections
        render RubyUI::Accordion.new(class: 'grid grid-cols-1 items-start gap-4 lg:grid-cols-2') do
          render_profile_section
          render_security_section
          render_notifications_section
          render_advanced_section
        end
      end

      def render_profile_section
        render Section.new(
          key: :profile,
          title: t('profiles.sections.profile.title'),
          summary: presenter.profile_summary,
          open: section_open?(:profile)
        ) do
          render_personal_info_card
          render ProfileSettings.new(person: presenter.person, account: presenter.account)
        end
      end

      def render_security_section
        render Section.new(
          key: :security,
          title: t('profiles.sections.security.title'),
          summary: presenter.security_summary,
          open: section_open?(:security)
        ) do
          render SecuritySectionContent.new(presenter:)
        end
      end

      def render_notifications_section
        render Section.new(
          key: :notifications,
          title: t('profiles.sections.notifications.title'),
          summary: presenter.notifications_summary,
          open: section_open?(:notifications)
        ) do
          render NotificationsCard.new(
            person: presenter.person,
            preference: presenter.notification_preference,
            managed_grants: presenter.managed_notification_grants
          )
        end
      end

      def render_advanced_section
        render Section.new(
          key: :advanced,
          title: t('profiles.sections.advanced.title'),
          summary: presenter.advanced_summary,
          open: section_open?(:advanced)
        ) do
          render AdvancedSectionContent.new(presenter:)
        end
      end

      def render_personal_info_card
        m3_card(
          class: 'overflow-hidden border border-border/70 shadow-elevation-1',
          data: { testid: 'profile-personal-info-card' }
        ) do
          render CardHeader.new do
            render(CardTitle.new { t('profiles.show.personal_information.title') })
            render(CardDescription.new { t('profiles.show.personal_information.description') })
          end
          render PersonalInfoContent.new(person: presenter.person, account: presenter.account)
        end
      end

      def section_open?(section)
        presenter.active_section == section.to_s
      end
    end
  end
end
