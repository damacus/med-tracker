# frozen_string_literal: true

module Views
  module Rodauth
    module LoginBrandSupport
      private

      def render_brand_panel
        div(data_login_panel: 'brand', class: brand_panel_classes) do
          render_brand_header
          render_benefit_list
          render_medication_illustration
        end
      end

      def render_brand_header
        div(class: 'flex items-center gap-5') do
          div(class: 'font-[Urbanist] text-6xl font-bold leading-none text-teal-600 dark:text-teal-400',
              aria_label: t('app.name')) do
            plain 'mt'
            span(class: '-ml-2 align-top text-4xl') { '+' }
          end
          span(class: 'text-xl font-bold text-foreground') { t('app.name') }
        end
      end

      def render_benefit_list
        div(class: 'mt-10 space-y-4') do
          login_benefits.each { |benefit| render_benefit_item(**benefit) }
        end
      end

      def render_benefit_item(title:, detail:, icon:, color_classes:)
        div(class: 'flex items-center gap-4') do
          div(class: "grid h-12 w-12 shrink-0 place-items-center rounded-lg border #{color_classes}") do
            render_benefit_icon(icon)
          end
          div(class: 'min-w-0') do
            p(class: 'text-sm font-bold text-foreground') { title }
            p(class: 'mt-1 text-sm font-medium text-on-surface-variant') { detail }
          end
        end
      end

      def render_benefit_icon(icon)
        case icon
        when :check_circle
          render Components::Icons::CheckCircle.new(size: 24)
        when :calendar
          render Components::Icons::Calendar.new(size: 24)
        when :activity
          render Components::Icons::Activity.new(size: 24)
        else
          render Components::Icons::Sparkles.new(size: 24)
        end
      end

      def render_medication_illustration
        div(data_login_illustration: 'medication', role: 'img',
            aria_label: t('sessions.login.medication_illustration_label'),
            class: 'relative mt-auto hidden h-48 overflow-hidden sm:block') do
          div(class: 'absolute -bottom-16 -left-16 h-44 w-44 rounded-full bg-blue-100/80 dark:bg-blue-950/50')
          div(class: 'absolute bottom-0 left-20 h-24 w-64 rounded-[50%] bg-teal-100/90 dark:bg-teal-900/45')
          div(class: 'absolute bottom-0 right-4 h-32 w-32 rounded-full bg-sky-100/80 dark:bg-sky-950/40')
          div(class: 'absolute bottom-2 left-44 h-24 w-20 rounded-lg bg-teal-500/75 shadow-elevation-3 dark:bg-teal-400/65') do
            div(class: 'mx-auto mt-3 h-5 w-16 rounded bg-teal-700/65 dark:bg-teal-300/60')
            div(class: 'mt-9 text-center text-5xl font-bold leading-none text-white') { '+' }
          end
          div(class: 'absolute bottom-3 left-34 h-5 w-11 rotate-[-18deg] rounded-full bg-white shadow-elevation-2')
          div(class: 'absolute bottom-0 left-28 h-5 w-11 rotate-[24deg] rounded-full bg-white shadow-elevation-2')
          div(class: 'absolute bottom-3 right-20 h-28 w-2 rotate-[28deg] rounded-full bg-teal-600/70 dark:bg-teal-300/70')
          div(class: 'absolute bottom-24 right-10 h-16 w-8 rotate-[31deg] rounded-[100%_0_100%_0] bg-teal-400/50 dark:bg-teal-300/40')
          div(class: 'absolute bottom-8 right-28 h-12 w-7 rotate-[-18deg] rounded-[0_100%_0_100%] bg-teal-500/50 dark:bg-teal-300/40')
        end
      end

      def login_benefits
        [
          {
            title: t('sessions.login.benefits.stay_on_track.title'),
            detail: t('sessions.login.benefits.stay_on_track.detail'),
            icon: :check_circle,
            color_classes: 'border-teal-200 bg-teal-50 text-teal-600 dark:border-teal-400/20 dark:bg-teal-400/10 dark:text-teal-300'
          },
          {
            title: t('sessions.login.benefits.schedule.title'),
            detail: t('sessions.login.benefits.schedule.detail'),
            icon: :calendar,
            color_classes: 'border-blue-200 bg-blue-50 text-blue-600 dark:border-blue-400/20 dark:bg-blue-400/10 dark:text-blue-300'
          },
          {
            title: t('sessions.login.benefits.progress.title'),
            detail: t('sessions.login.benefits.progress.detail'),
            icon: :activity,
            color_classes: 'border-emerald-200 bg-emerald-50 text-emerald-600 dark:border-emerald-400/20 dark:bg-emerald-400/10 dark:text-emerald-300'
          },
          {
            title: t('sessions.login.benefits.insights.title'),
            detail: t('sessions.login.benefits.insights.detail'),
            icon: :sparkles,
            color_classes: 'border-purple-200 bg-purple-50 text-purple-600 dark:border-purple-400/20 dark:bg-purple-400/10 dark:text-purple-300'
          }
        ]
      end
    end
  end
end
