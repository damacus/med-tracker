# frozen_string_literal: true

module Views
  module Profiles
    class Show < Views::Base
      SECTION_KEYS = %i[profile security notifications advanced].freeze

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
        render RubyUI::Tabs.new(default: presenter.active_section, class: 'space-y-4') do
          render_section_tabs
          render_section_panels
        end
      end

      def render_section_tabs
        nav(aria: { label: t('profiles.show.title') }) do
          render RubyUI::TabsList.new(
            role: 'tablist',
            class: 'flex h-auto w-full rounded-shape-lg border border-outline-variant/70 ' \
                   'bg-surface-container p-1 shadow-elevation-1'
          ) do
            SECTION_KEYS.each { |key| render_section_tab(key) }
          end
        end
      end

      def render_section_tab(key)
        active = presenter.active_section == key.to_s
        render RubyUI::TabsTrigger.new(
          value: key.to_s,
          id: "profile-tab-#{key}",
          role: 'tab',
          aria: {
            controls: "profile-#{key}-panel",
            selected: active.to_s
          },
          data: {
            state: active ? 'active' : 'inactive',
            testid: 'profile-section-tab',
            profile_section: key
          },
          class: 'min-h-12 min-w-0 flex-1 rounded-shape-md px-1 text-xs font-semibold sm:px-3 sm:text-sm ' \
                 'data-[state=active]:bg-surface data-[state=active]:text-primary ' \
                 'data-[state=active]:shadow-elevation-1'
        ) { t("profiles.sections.#{key}.title") }
      end

      def render_section_panels
        SECTION_KEYS.each do |key|
          render RubyUI::TabsContent.new(
            value: key.to_s,
            id: "profile-#{key}-panel",
            role: 'tabpanel',
            aria: { labelledby: "profile-tab-#{key}" },
            hidden: presenter.active_section != key.to_s,
            tabindex: 0,
            data: { testid: 'profile-section-panel' },
            class: 'mt-0'
          ) { render_section(key) }
        end
      end

      def render_section(key)
        case key
        when :security then render_security_section
        when :notifications then render_notifications_section
        when :advanced then render_advanced_section
        else render_profile_section
        end
      end

      def render_profile_section
        render Section.new(
          key: :profile,
          title: t('profiles.sections.profile.title'),
          summary: presenter.profile_summary
        ) do
          div(class: 'grid gap-5 lg:grid-cols-[minmax(0,1.1fr)_minmax(20rem,0.9fr)] lg:items-start') do
            render_personal_info_card
            render ProfileSettings.new(person: presenter.person, account: presenter.account)
          end
        end
      end

      def render_security_section
        render Section.new(
          key: :security,
          title: t('profiles.sections.security.title'),
          summary: presenter.security_summary
        ) do
          render SecuritySectionContent.new(presenter:)
        end
      end

      def render_notifications_section
        render Section.new(
          key: :notifications,
          title: t('profiles.sections.notifications.title'),
          summary: presenter.notifications_summary
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
          summary: presenter.advanced_summary
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
    end
  end
end
