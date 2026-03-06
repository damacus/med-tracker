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
        render Card.new do
          render CardHeader.new do
            render(CardTitle.new { t('profiles.appearance.title') })
            render(CardDescription.new { t('profiles.appearance.description') })
          end
          render_card_content
        end
      end

      private

      def render_card_content
        render CardContent.new(class: 'space-y-8', data: { controller: 'appearance' }) do
          render_appearance_section
          render_palette_section
        end
      end

      def render_appearance_section
        div(class: 'space-y-4') do
          p(class: 'text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground') do
            t('profiles.appearance.mode_label')
          end
          div(class: 'inline-flex flex-wrap gap-2 rounded-2xl border border-border bg-muted/70 p-1') do
            APPEARANCES.each do |appearance|
              render_appearance_option(appearance)
            end
          end
        end
      end

      def render_palette_section
        div(class: 'space-y-4') do
          p(class: 'text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground') do
            t('profiles.theme_picker.title')
          end
          p(class: 'text-sm text-muted-foreground') do
            t('profiles.theme_picker.description')
          end
          div(class: 'flex flex-wrap gap-4') do
            THEMES.each do |theme|
              render_theme_option(theme)
            end
          end
        end
      end

      def render_appearance_option(appearance)
        button(
          type: 'button',
          aria_pressed: 'false',
          class: 'inline-flex min-h-11 items-center justify-center gap-2 rounded-xl px-4 py-2 text-sm font-semibold text-muted-foreground transition-all hover:bg-background hover:text-foreground focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background data-[active=true]:bg-background data-[active=true]:text-foreground data-[active=true]:shadow-sm',
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
          class: 'group flex flex-col items-center gap-2 rounded-2xl px-2 py-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-background',
          data: {
            action: 'click->appearance#switchTheme',
            theme: theme[:id]
          }
        ) do
          div(
            class: "w-12 h-12 rounded-full #{theme[:color]} border border-border/80 shadow-sm shadow-black/5 transition-transform group-hover:scale-110 data-[active=true]:ring-2 data-[active=true]:ring-primary data-[active=true]:ring-offset-2 data-[active=true]:ring-offset-background",
            data: { theme_swatch: true, active: false }
          )
          span(class: 'text-[10px] font-medium uppercase tracking-tighter text-muted-foreground group-data-[active=true]:text-foreground') do
            t("profiles.theme_picker.themes.#{theme[:id].underscore}")
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
