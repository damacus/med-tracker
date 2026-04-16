# frozen_string_literal: true

module Components
  module Dashboard
    # Renders the dashboard quick actions section
    class QuickActions < Components::Base
      def view_template
        div(class: 'space-y-6') do
          m3_heading(
            variant: :title_large, level: 2,
            class: 'font-bold tracking-tight'
          ) do
            t('dashboard.quick_actions.title')
          end
          div(class: 'grid grid-cols-1 gap-3') do
            action_links.each do |label, url, icon|
              m3_link(
                href: url,
                variant: :tonal,
                size: :lg,
                class: 'w-full py-6 rounded-xl font-bold transition-all shadow-sm'
              ) do
                render icon.new(size: 20, class: 'mr-2') if icon
                plain label
              end
            end
          end
        end
      end

      private

      def action_links
        [
          [t('dashboard.quick_actions.add_medication'), add_medication_path, Icons::PlusCircle],
          [t('dashboard.quick_actions.add_person'), new_person_path, Icons::User]
        ]
      end
    end
  end
end
