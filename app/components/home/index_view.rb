# frozen_string_literal: true

module Components
  module Home
    class IndexView < Components::Base
      def view_template
        div(class: 'container mx-auto px-4 py-8') do
          div(class: 'mb-8 text-center') do
            h1(class: 'text-4xl font-bold text-slate-900 mb-2') { 'Medicine Tracker' }
            p(class: 'text-lg text-slate-600') { 'Welcome to your personal medicine tracking application.' }
          end

          div(class: 'grid grid-cols-1 md:grid-cols-2 gap-6 max-w-4xl mx-auto') do
            medicines_card
            people_card
          end
        end
      end

      private

      def medicines_card
        a(href: medicines_path, class: 'block transition-transform hover:scale-105') do
          Card(class: 'h-full') do
            CardHeader do
              div(class: 'w-12 h-12 rounded-xl flex items-center justify-center bg-blue-100 text-blue-700 mb-2') do
                render_medicine_icon
              end
            end
            CardContent(class: 'space-y-2') do
              CardTitle(class: 'text-2xl') { 'Medicines' }
              CardDescription { 'Manage your medicine inventory, track stock levels, and view dosage information' }
            end
          end
        end
      end

      def people_card
        a(href: people_path, class: 'block transition-transform hover:scale-105') do
          Card(class: 'h-full') do
            CardHeader do
              div(class: 'w-12 h-12 rounded-xl flex items-center justify-center bg-violet-100 text-violet-700 mb-2') do
                render_people_icon
              end
            end
            CardContent(class: 'space-y-2') do
              CardTitle(class: 'text-2xl') { 'People' }
              CardDescription { 'Manage people and their prescriptions, track medication schedules' }
            end
          end
        end
      end

      def render_medicine_icon
        svg(
          xmlns: 'http://www.w3.org/2000/svg',
          width: '24',
          height: '24',
          viewBox: '0 0 24 24',
          fill: 'none',
          stroke: 'currentColor',
          stroke_width: '2',
          stroke_linecap: 'round',
          stroke_linejoin: 'round'
        ) do |s|
          s.path(d: 'M10.5 20.5 10 21a2 2 0 0 1-2.828 0L4.343 18.172a2 2 0 0 1 0-2.828l.5-.5')
          s.path(d: 'm7 17-5-5')
          s.path(
            d: 'M13.5 3.5 14 3a2 2 0 0 1 2.828 0l2.829 2.828a2 2 0 0 1 0 2.829l-.5.5'
          )
          s.path(d: 'm17 7 5 5')
          s.path(d: 'M9 11 4 6')
          s.path(d: 'm13 15 5-5')
          s.circle(cx: '12', cy: '12', r: '2')
        end
      end

      def render_people_icon
        svg(
          xmlns: 'http://www.w3.org/2000/svg',
          width: '24',
          height: '24',
          viewBox: '0 0 24 24',
          fill: 'none',
          stroke: 'currentColor',
          stroke_width: '2',
          stroke_linecap: 'round',
          stroke_linejoin: 'round'
        ) do |s|
          s.path(d: 'M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2')
          s.circle(cx: '9', cy: '7', r: '4')
          s.path(d: 'M22 21v-2a4 4 0 0 0-3-3.87')
          s.path(d: 'M16 3.13a4 4 0 0 1 0 7.75')
        end
      end
    end
  end
end
