# frozen_string_literal: true

module Views
  module Profiles
    class ThemePickerCard < Views::Base
      APPEARANCES = [
        { id: 'light', icon: 'sun' },
        { id: 'dark', icon: 'moon' },
        { id: 'system', icon: 'sparkles' }
      ].freeze

      THEMES = [
        { id: 'default', name: 'Standard', color: 'bg-slate-900' },
        { id: 'serene-sage', name: 'Serene Sage', color: 'bg-[#7DAA92]' },
        { id: 'modern-clinical', name: 'Modern Clinical', color: 'bg-[#0066FF]' },
        { id: 'warm-earth', name: 'Warm Earth', color: 'bg-[#E07A5F]' },
        { id: 'deep-lavender', name: 'Deep Lavender', color: 'bg-[#9B5DE5]' },
        { id: 'forest-care', name: 'Forest Care', color: 'bg-[#2D6A4F]' },
        { id: 'sunset-support', name: 'Sunset Support', color: 'bg-[#F28482]' },
        { id: 'tech-indigo', name: 'Tech Indigo', color: 'bg-[#4361EE]' },
        { id: 'soft-rose', name: 'Soft Rose', color: 'bg-[#E5989B]' },
        { id: 'minty-fresh', name: 'Minty Fresh', color: 'bg-[#06D6A0]' },
        { id: 'minimalist-monochrome', name: 'Minimalist', color: 'bg-[#1A1A1A]' }
      ].freeze

      def view_template
        render Card.new(class: 'overflow-hidden rounded-[2rem] border border-border/70 bg-[radial-gradient(circle_at_top_left,_rgba(244,238,229,0.88),_transparent_28%),linear-gradient(145deg,_rgba(255,255,255,0.98),_rgba(244,247,251,0.92))] shadow-[0_26px_60px_-36px_rgba(15,23,42,0.42)] dark:bg-[radial-gradient(circle_at_top_left,_rgba(72,187,152,0.12),_transparent_24%),linear-gradient(145deg,_rgba(20,25,34,0.98),_rgba(28,35,46,0.95))]') do
          render CardHeader.new(class: 'relative overflow-hidden border-b border-border/60 pb-6') do
            div(class: 'absolute right-0 top-0 hidden h-24 w-24 rounded-full bg-primary/8 blur-2xl sm:block')
            p(class: "relative mb-3 text-[0.7rem] font-semibold uppercase tracking-[0.34em] text-muted-foreground [font-family:'Outfit',sans-serif]") do
              'Appearance Studio'
            end
            render(CardTitle.new(class: "relative text-2xl tracking-tight [font-family:'Outfit',sans-serif] sm:text-3xl") do
              t('profiles.appearance.title')
            end)
            render(CardDescription.new(class: 'relative max-w-2xl text-sm leading-6') do
              t('profiles.appearance.description')
            end)
          end
          render_card_content
        end
      end

      private

      def render_card_content
        render CardContent.new(class: 'space-y-8 px-5 py-6 sm:px-6', data: { controller: 'appearance' }) do
          render_appearance_section
          render_palette_section
        end
      end

      def render_appearance_section
        div(class: 'grid gap-5 lg:grid-cols-[minmax(0,15rem)_minmax(0,1fr)] lg:items-start') do
          div(class: 'space-y-2') do
            p(class: "text-[0.7rem] font-semibold uppercase tracking-[0.28em] text-muted-foreground [font-family:'Outfit',sans-serif]") do
              t('profiles.appearance.mode_label')
            end
            p(class: 'text-sm leading-6 text-muted-foreground') do
              'Switch the overall light balance first, then fine-tune the palette below.'
            end
          end
          div(class: 'grid grid-cols-1 gap-2 rounded-[1.5rem] border border-border/70 bg-muted/55 p-2 sm:grid-cols-3') do
            APPEARANCES.each_with_index do |appearance, index|
              render_appearance_option(appearance, index)
            end
          end
        end
      end

      def render_palette_section
        div(class: 'space-y-4') do
          p(class: "text-[0.7rem] font-semibold uppercase tracking-[0.28em] text-muted-foreground [font-family:'Outfit',sans-serif]") do
            t('profiles.theme_picker.title')
          end
          p(class: 'text-sm text-muted-foreground') do
            t('profiles.theme_picker.description')
          end
          div(class: 'grid grid-cols-2 gap-3 sm:grid-cols-3 xl:grid-cols-4') do
            THEMES.each do |theme|
              render_theme_option(theme)
            end
          end
        end
      end

      def render_appearance_option(appearance, index)
        button(
          type: 'button',
          aria_pressed: 'false',
          class: 'inline-flex min-h-14 items-center justify-center gap-2 rounded-[1.1rem] border border-transparent bg-background/75 px-4 py-3 text-sm font-semibold text-muted-foreground shadow-[0_10px_25px_-22px_rgba(15,23,42,0.65)] transition-all duration-300 hover:-translate-y-0.5 hover:border-border/80 hover:text-foreground focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background data-[active=true]:border-primary/30 data-[active=true]:bg-background data-[active=true]:text-foreground data-[active=true]:shadow-[0_14px_30px_-24px_rgba(15,23,42,0.75)]',
          style: "animation-delay: #{index * 70}ms",
          data: {
            action: 'click->appearance#switchAppearance',
            appearance: appearance[:id]
          }
        ) do
          render_appearance_icon(appearance[:icon])
          span { t("profiles.appearance.modes.#{appearance[:id]}") }
        end
      end

      def render_theme_option(theme)
        button(
          type: 'button',
          aria_pressed: 'false',
          class: 'group flex min-h-[11rem] w-full flex-col justify-between rounded-[1.5rem] border border-border/65 bg-white/80 p-3 text-left shadow-[0_16px_34px_-28px_rgba(15,23,42,0.6)] transition-all duration-300 hover:-translate-y-1 hover:border-primary/35 hover:shadow-[0_26px_40px_-30px_rgba(15,23,42,0.75)] focus:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-background dark:bg-white/5',
          data: {
            action: 'click->appearance#switchTheme',
            theme: theme[:id]
          }
        ) do
          div(class: 'space-y-4') do
            div(
              class: "h-16 w-16 rounded-full #{theme[:color]} border border-white/70 shadow-[0_14px_28px_-18px_rgba(15,23,42,0.75)] transition-transform duration-300 group-hover:scale-110 data-[active=true]:scale-105 data-[active=true]:ring-2 data-[active=true]:ring-primary data-[active=true]:ring-offset-2 data-[active=true]:ring-offset-background",
              data: { theme_swatch: true, active: false }
            )
            div do
              span(class: "block text-sm font-semibold uppercase tracking-[0.12em] leading-tight text-foreground [font-family:'Outfit',sans-serif]") do
                t("profiles.theme_picker.themes.#{theme[:id].underscore}")
              end
            end
          end
          span(class: 'inline-flex items-center text-[0.68rem] font-semibold uppercase tracking-[0.22em] text-muted-foreground transition-colors group-hover:text-foreground') do
            'Apply palette'
          end
        end
      end

      def render_appearance_icon(icon)
        case icon
        when 'sun'
          render Components::Icons::Sun.new(size: 16)
        when 'moon'
          render Components::Icons::Moon.new(size: 16)
        else
          render Components::Icons::Sparkles.new(size: 16)
        end
      end
    end
  end
end
