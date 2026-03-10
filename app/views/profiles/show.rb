# frozen_string_literal: true

module Views
  module Profiles
    class Show < Views::Base
      include Phlex::Rails::Helpers::LinkTo
      include Phlex::Rails::Helpers::Routes
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::ButtonTo

      attr_reader :person, :account

      def initialize(person:, account:)
        super()
        @person = person
        @account = account
      end

      def view_template
        div(class: 'container mx-auto max-w-6xl px-4 py-8 pb-24 md:pb-8') do
          render_header
          render_profile_sections
        end
      end

      private

      def render_header
        div(class: 'relative mb-8 overflow-hidden rounded-[2rem] border border-border/70 bg-[radial-gradient(circle_at_top_left,_rgba(125,170,146,0.2),_transparent_32%),linear-gradient(135deg,_rgba(255,255,255,0.96),_rgba(244,238,229,0.9))] px-6 py-8 shadow-[0_24px_60px_-28px_rgba(15,23,42,0.28)] dark:bg-[radial-gradient(circle_at_top_left,_rgba(125,170,146,0.16),_transparent_32%),linear-gradient(135deg,_rgba(24,28,39,0.96),_rgba(31,38,48,0.94))] sm:px-8') do
          div(class: 'relative flex flex-col gap-6 lg:flex-row lg:items-center lg:justify-between') do
            div(class: 'max-w-2xl') do
              p(class: "mb-3 text-[0.7rem] font-semibold uppercase tracking-[0.34em] text-muted-foreground [font-family:'Outfit',sans-serif]") do
                'Account Atelier'
              end
              h1(class: "text-4xl font-semibold tracking-tight text-foreground [font-family:'Outfit',sans-serif] sm:text-5xl") do
                'My Profile'
              end
              p(class: 'mt-3 max-w-xl text-sm leading-6 text-muted-foreground sm:text-base') do
                'Shape your account, safety preferences, and notification rhythm from one calm control room.'
              end
            end
            div(class: 'grid gap-3 sm:grid-cols-2 lg:w-[24rem]') do
              render_header_stat('Profile name', person.name)
              render_header_stat('Sign-in email', account.email)
            end
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
          render DangerZoneCard.new
          render VersionInfo.new
        end
      end

      def render_personal_info_card
        render Card.new(class: 'overflow-hidden rounded-[2rem] border border-border/70 bg-card/95 shadow-[0_18px_45px_-32px_rgba(15,23,42,0.45)]') do
          render CardHeader.new do
            render(CardTitle.new { 'Personal Information' })
            render(CardDescription.new { 'Your basic profile information' })
          end
          render PersonalInfoContent.new(person: person, account: account)
        end
      end

      def render_header_stat(label, value)
        div(class: 'rounded-[1.35rem] border border-white/50 bg-white/70 px-4 py-4 shadow-[0_10px_30px_-24px_rgba(15,23,42,0.55)] backdrop-blur dark:border-white/10 dark:bg-white/5') do
          p(class: 'text-[0.68rem] font-semibold uppercase tracking-[0.22em] text-muted-foreground') { label }
          p(class: 'mt-2 truncate text-sm font-semibold text-foreground sm:text-base') { value.presence || 'Not set' }
        end
      end
    end
  end
end
