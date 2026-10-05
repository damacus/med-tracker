# frozen_string_literal: true

module Components
  module Home
    class IndexView < Components::Base
      def view_template
        div(class: 'container mx-auto px-4 py-8') do
          div(class: 'mb-8 text-center') do
            m3_heading(level: 1, class: 'mb-2') { t('home.title') }
            m3_text(size: '4', weight: 'muted') { t('home.description') }
          end

          div(class: 'grid grid-cols-1 md:grid-cols-2 gap-6 max-w-4xl mx-auto') do
            medications_card
            people_card
          end
        end
      end

      private

      def medications_card
        Link(href: medications_path, variant: :ghost, class: 'block transition-transform hover:scale-105 h-auto p-0') do
          m3_card(class: 'h-full') do
            CardHeader do
              div(
                class: 'w-12 h-12 rounded-xl flex items-center justify-center ' \
                       'bg-primary-container text-on-primary-container mb-2'
              ) do
                render_medication_icon
              end
            end
            CardContent(class: 'space-y-2') do
              m3_heading(level: 2, size: '6', class: 'font-semibold leading-none tracking-tight') do
                t('home.cards.medications.title')
              end
              CardDescription { t('home.cards.medications.description') }
            end
          end
        end
      end

      def people_card
        Link(href: people_path, variant: :ghost, class: 'block transition-transform hover:scale-105 h-auto p-0') do
          m3_card(class: 'h-full') do
            CardHeader do
              div(
                class: 'w-12 h-12 rounded-xl flex items-center justify-center ' \
                       'bg-secondary-container text-on-secondary-container mb-2'
              ) do
                render_people_icon
              end
            end
            CardContent(class: 'space-y-2') do
              m3_heading(level: 2, size: '6', class: 'font-semibold leading-none tracking-tight') do
                t('home.cards.people.title')
              end
              CardDescription { t('home.cards.people.description') }
            end
          end
        end
      end

      def render_medication_icon
        render Icons::Pill.new(size: 24)
      end

      def render_people_icon
        render Icons::Users.new(size: 24)
      end
    end
  end
end
