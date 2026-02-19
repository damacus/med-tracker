# frozen_string_literal: true

module Views
  module Profiles
    class ThemePickerCard < Views::Base
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
            render(CardTitle.new { t('profiles.theme_picker.title') })
            render(CardDescription.new { t('profiles.theme_picker.description') })
          end
          render CardContent.new do
            div(class: 'flex flex-wrap gap-4', data_controller: 'theme-switcher') do
              THEMES.each do |theme|
                render_theme_option(theme)
              end
            end
          end
        end
      end

      private

      def render_theme_option(theme)
        button(
          class: 'group flex flex-col items-center gap-2 focus:outline-none',
          data_action: 'click->theme-switcher#switch',
          data_theme: theme[:id]
        ) do
          div(
            class: "w-12 h-12 rounded-full #{theme[:color]} border-2 border-white shadow-sm transition-transform group-hover:scale-110"
          )
          span(class: 'text-[10px] font-medium text-slate-500 uppercase tracking-tighter') { t("profiles.theme_picker.themes.#{theme[:id].underscore}") }
        end
      end
    end
  end
end
