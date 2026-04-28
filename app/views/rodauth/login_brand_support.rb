# frozen_string_literal: true

module Views
  module Rodauth
    module LoginBrandSupport
      private

      def render_brand_panel
        div(data_login_panel: 'brand', class: brand_panel_classes) do
          render_brand_header
          render_brand_welcome
          render_benefit_list
          render_medication_illustration
        end
      end

      def render_brand_header
        div(class: 'flex items-center justify-center gap-4 md:justify-start md:gap-5') do
          render_mt_logo
          span(class: 'text-xl font-bold text-foreground') { t('app.name') }
        end
      end

      def render_mt_logo
        svg(width: '96', height: '48', viewbox: '0 0 96 48', fill: 'none', xmlns: 'http://www.w3.org/2000/svg',
            data_login_logo: 'mt', aria_label: t('app.name'), role: 'img',
            class: 'h-10 w-20 md:h-12 md:w-24') do |s|
          logo_path_attrs.each { |attrs| s.path(**attrs) }
        end
      end

      def logo_path_attrs
        [
          {
            d: 'M8 40V18C8 11.4 13.4 6 20 6C26.6 6 32 11.4 32 18V40',
            stroke: '#14A99A', stroke_width: '6', stroke_linecap: 'round'
          },
          {
            d: 'M32 40V18C32 11.4 37.4 6 44 6C50.6 6 56 11.4 56 18V40',
            stroke: '#14A99A', stroke_width: '6', stroke_linecap: 'round'
          },
          { d: 'M64 6V40', stroke: '#14A99A', stroke_width: '6', stroke_linecap: 'round' },
          { d: 'M58 18H80', stroke: '#14A99A', stroke_width: '6', stroke_linecap: 'round' },
          { d: 'M78 4V14', stroke: '#14A99A', stroke_width: '5', stroke_linecap: 'round' }
        ]
      end

      def render_brand_welcome
        div(class: 'space-y-2 text-center md:mt-10 md:space-y-3 md:text-left') do
          h1(class: 'text-2xl font-bold leading-tight text-foreground sm:text-4xl md:text-5xl') { t('sessions.login.heading') }
          p(class: 'text-sm font-medium text-on-surface-variant md:text-base') { t('sessions.login.subheading') }
        end
      end

      def render_benefit_list
        div(data_login_benefits: true, class: 'hidden md:mt-7 md:block md:space-y-4') do
          login_benefits.each { |benefit| render_benefit_item(**benefit) }
        end
      end

      def render_benefit_item(title:, detail:, icon:, color_classes:)
        div(class: 'flex items-center gap-4') do
          div(class: "grid h-12 w-12 shrink-0 place-items-center #{benefit_icon_frame_classes(icon, color_classes)}") do
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
        when :heart_check
          render_heart_check_icon
        when :schedule_calendar
          render_schedule_calendar_icon
        when :progress_path_pin
          render_progress_path_pin_icon
        when :insights_dot_grid_heart
          render_insights_dot_grid_heart_icon
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

      def benefit_icon_frame_classes(icon, color_classes)
        return '' if custom_benefit_icon?(icon)

        "rounded-lg border #{color_classes}"
      end

      def custom_benefit_icon?(icon)
        %i[heart_check schedule_calendar progress_path_pin insights_dot_grid_heart].include?(icon)
      end

      def render_heart_check_icon
        svg(width: '48', height: '48', viewbox: '0 0 48 48', fill: 'none', xmlns: 'http://www.w3.org/2000/svg',
            data_login_benefit_icon: 'stay-on-track', aria_hidden: 'true') do |s|
          s.rect(width: '48', height: '48', rx: '14', fill: '#E0F7F4')
          s.path(
            d: 'M24 35C23.4 35 13 28.7 13 20.2C13 15.8 16 13 19.7 13C22 13 23.5 14.2 24 15.1C24.5 14.2 26 13 28.3 13C32 13 35 15.8 35 20.2C35 28.7 24.6 35 24 35Z',
            stroke: '#109E91', stroke_width: '3', stroke_linejoin: 'round'
          )
          s.path(
            d: 'M18.5 22.8L22.4 26.7L30 19.2',
            stroke: '#109E91', stroke_width: '3', stroke_linecap: 'round', stroke_linejoin: 'round'
          )
        end
      end

      def render_schedule_calendar_icon
        svg(width: '48', height: '48', viewbox: '0 0 48 48', fill: 'none', xmlns: 'http://www.w3.org/2000/svg',
            data_login_benefit_icon: 'schedule', aria_hidden: 'true') do |s|
          s.rect(width: '48', height: '48', rx: '14', fill: '#E7F0FF')
          s.svg(x: '12', y: '12', width: '24', height: '24', viewbox: '0 -960 960 960',
                xmlns: 'http://www.w3.org/2000/svg') do |icon|
            icon.path(d: material_calendar_check_path, fill: '#2F7DE1')
          end
        end
      end

      def material_calendar_check_path
        'M200-80q-33 0-56.5-23.5T120-160v-560q0-33 23.5-56.5T200-800h40v-80h80v80h320v-80h80v80h40q33 ' \
          '0 56.5 23.5T840-720v255l-80 80v-175H200v400h248l80 80H200Zm0-560h560v-80H200v80Zm0 ' \
          '0v-80 80Zm462 580L520-202l56-56 85 85 170-170 56 57L662-60Z'
      end

      def render_progress_path_pin_icon
        svg(width: '48', height: '48', viewbox: '0 0 48 48', fill: 'none', xmlns: 'http://www.w3.org/2000/svg',
            data_login_benefit_icon: 'progress', aria_hidden: 'true') do |s|
          s.rect(width: '48', height: '48', rx: '14', fill: '#DFF7F1')
          s.path(
            d: 'M21 14C16.6 14 13 17.4 13 21.8C13 27.8 21 35 21 35C21 35 29 27.8 29 21.8C29 17.4 25.4 14 21 14Z',
            fill: '#109E91'
          )
          s.path(d: 'M21 18.5V25', stroke: 'white', stroke_width: '3', stroke_linecap: 'round')
          s.path(d: 'M17.7 21.8H24.3', stroke: 'white', stroke_width: '3', stroke_linecap: 'round')
          s.path(
            d: 'M28 34C31 35.5 34 35.2 36 33.5C38.1 31.7 38.8 29.3 39 27',
            stroke: '#109E91', stroke_width: '3', stroke_linecap: 'round', stroke_dasharray: '1 6'
          )
          s.circle(cx: '39', cy: '25', r: '3', fill: '#109E91')
        end
      end

      def render_insights_dot_grid_heart_icon
        svg(width: '48', height: '48', viewbox: '0 0 48 48', fill: 'none', xmlns: 'http://www.w3.org/2000/svg',
            data_login_benefit_icon: 'insights', aria_hidden: 'true') do |s|
          s.rect(width: '48', height: '48', rx: '14', fill: '#F2E7FF')
          s.svg(x: '12', y: '12', width: '24', height: '24', viewbox: '0 -960 960 960',
                xmlns: 'http://www.w3.org/2000/svg') do |icon|
            icon.path(d: material_insights_search_path, fill: '#9A5CF7')
          end
        end
      end

      def material_insights_search_path
        'M400-320q100 0 170-70t70-170q0-100-70-170t-170-70q-100 0-170 70t-70 170q0 100 70 170t170 ' \
          '70Zm-40-120v-280h80v280h-80Zm-140 0v-200h80v200h-80Zm280 0v-160h80v160h-80ZM824-80 ' \
          '597-307q-41 32-91 49.5T400-240q-134 0-227-93T80-560q0-134 93-227t227-93q134 0 227 ' \
          '93t93 227q0 56-17.5 106T653-363l227 227-56 56Z'
      end

      def render_medication_illustration
        div(data_login_illustration: 'medication', role: 'img',
            aria_label: t('sessions.login.medication_illustration_label'),
            class: 'login-med-illustration') do
          render_medication_illustration_image(
            desktop_asset_path: 'auth/login-med-illustration-light-desktop.png',
            mobile_asset_path: 'auth/login-med-illustration-light-mobile.png',
            variant_class: 'login-med-illustration__picture--light'
          )
          render_medication_illustration_image(
            desktop_asset_path: 'auth/login-med-illustration-dark-desktop.png',
            mobile_asset_path: 'auth/login-med-illustration-dark-mobile.png',
            variant_class: 'login-med-illustration__picture--dark'
          )
        end
      end

      def render_medication_illustration_image(desktop_asset_path:, mobile_asset_path:, variant_class:)
        picture(class: "login-med-illustration__picture #{variant_class}") do
          source(
            media: '(max-width: 520px)',
            srcset: view_context.image_path(mobile_asset_path)
          )
          img(
            src: view_context.image_path(desktop_asset_path),
            alt: '',
            aria_hidden: 'true',
            loading: 'eager',
            class: 'login-med-illustration__image'
          )
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
