# frozen_string_literal: true

module Views
  module Rodauth
    module LoginBenefitsSupport
      private

      def render_benefit_list
        div(data_login_benefits: true, class: 'hidden md:mt-7 md:block md:space-y-4') do
          login_benefits.each { |benefit| render_benefit_item(**benefit) }
        end
      end

      def render_benefit_item(title:, detail:, icon:, color_classes:)
        div(class: 'flex items-center gap-4') do
          render Components::Auth::BenefitIconTile.new(icon: icon, color_classes: color_classes)
          div(class: 'min-w-0') do
            p(class: 'text-sm font-bold text-foreground') { title }
            p(class: 'mt-1 text-sm font-medium text-on-surface-variant') { detail }
          end
        end
      end

      def login_benefits
        [
          {
            title: t('sessions.login.benefits.stay_on_track.title'),
            detail: t('sessions.login.benefits.stay_on_track.detail'),
            icon: :heart_check,
            color_classes: 'border-teal-200 bg-teal-50 text-teal-600 dark:border-teal-400/20 dark:bg-teal-400/10 dark:text-teal-300'
          },
          {
            title: t('sessions.login.benefits.schedule.title'),
            detail: t('sessions.login.benefits.schedule.detail'),
            icon: :schedule_calendar,
            color_classes: 'border-blue-200 bg-blue-50 text-blue-600 dark:border-blue-400/20 dark:bg-blue-400/10 dark:text-blue-300'
          },
          {
            title: t('sessions.login.benefits.progress.title'),
            detail: t('sessions.login.benefits.progress.detail'),
            icon: :progress_path_pin,
            color_classes: 'border-emerald-200 bg-emerald-50 text-emerald-600 dark:border-emerald-400/20 dark:bg-emerald-400/10 dark:text-emerald-300'
          },
          {
            title: t('sessions.login.benefits.insights.title'),
            detail: t('sessions.login.benefits.insights.detail'),
            icon: :insights_dot_grid_heart,
            color_classes: 'border-purple-200 bg-purple-50 text-purple-600 dark:border-purple-400/20 dark:bg-purple-400/10 dark:text-purple-300'
          }
        ]
      end
    end
  end
end
