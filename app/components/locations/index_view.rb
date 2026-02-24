# frozen_string_literal: true

module Components
  module Locations
    class IndexView < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::Pluralize

      attr_reader :locations

      def initialize(locations:)
        @locations = locations
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-12 max-w-6xl', data: { testid: 'locations-list' }) do
          render_header
          render_locations_grid
        end
      end

      private

      def render_header
        div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 mb-12') do
          div do
            Text(size: '2', weight: 'muted', class: 'uppercase tracking-widest mb-1 block font-bold') do
              'Manage Locations'
            end
            Heading(level: 1, size: '8', class: 'font-extrabold tracking-tight') { 'Locations' }
          end
          Link(
            href: new_location_path,
            variant: :primary,
            size: :lg,
            class: 'rounded-2xl font-bold text-sm shadow-lg shadow-primary/20'
          ) do
            span { 'Add Location' }
          end
        end
      end

      def render_locations_grid
        div(class: 'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8', id: 'locations') do
          locations.each do |location|
            render_location_card(location)
          end
        end
      end

      def render_location_card(location)
        Card(
          id: "location_#{location.id}",
          class: 'h-full flex flex-col border-none shadow-[0_8px_30px_rgb(0,0,0,0.04)] bg-white ' \
                 'rounded-[2.5rem] transition-all duration-300 hover:scale-[1.02] hover:shadow-xl ' \
                 'group overflow-hidden'
        ) do
          CardHeader(class: 'pb-4 pt-8 px-8') do
            div(class: 'flex justify-between items-start mb-4') do
              render_location_icon
              Badge(variant: :outline) { pluralize(location.medicines.size, 'medicine') }
            end
            Heading(level: 2, size: '5', class: 'font-bold tracking-tight') { location.name }
          end

          CardContent(class: 'flex-grow space-y-4 px-8 pb-4') do
            if location.description.present?
              Text(size: '2', class: 'text-slate-400 line-clamp-2 leading-relaxed') { location.description }
            end

            if location.members.any?
              div(class: 'pt-4 border-t border-slate-50') do
                Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-slate-400 mb-2 block') do
                  'Members'
                end
                div(class: 'flex flex-wrap gap-1') do
                  location.members.each do |member|
                    Badge(variant: :secondary, class: 'text-xs') { member.name }
                  end
                end
              end
            end
          end

          CardFooter(class: 'px-8 pb-8 pt-2 mt-auto') do
            render_location_actions(location)
          end
        end
      end

      def render_location_icon
        div(
          class: 'w-12 h-12 rounded-2xl bg-slate-50 flex items-center justify-center text-slate-400 ' \
                 'group-hover:text-primary group-hover:bg-primary/5 transition-all'
        ) do
          render Icons::Home.new(size: 24)
        end
      end

      def render_location_actions(location)
        div(class: 'flex items-center gap-2 w-full') do
          Link(
            href: location_path(location),
            variant: :outline,
            size: :sm,
            class: 'flex-1 rounded-xl py-5 border-slate-100 bg-white hover:bg-slate-50 text-slate-600'
          ) do
            'View'
          end
          Link(
            href: edit_location_path(location),
            variant: :outline,
            size: :sm,
            class: 'rounded-xl w-10 h-10 p-0 border-slate-100 bg-white hover:bg-slate-50 text-slate-400'
          ) do
            svg(
              xmlns: 'http://www.w3.org/2000/svg',
              class: 'w-4 h-4',
              fill: 'none',
              viewBox: '0 0 24 24',
              stroke: 'currentColor'
            ) do |s|
              s.path(
                stroke_linecap: 'round',
                stroke_linejoin: 'round',
                stroke_width: '2',
                d: 'M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z'
              )
            end
          end
          render_delete_dialog(location)
        end
      end

      def render_delete_dialog(location)
        AlertDialog do
          AlertDialogTrigger do
            Button(variant: :ghost, size: :sm,
                   class: 'rounded-xl w-10 h-10 p-0 text-slate-300 hover:text-destructive hover:bg-destructive/5') do
              render Icons::Trash.new(size: 18)
            end
          end
          AlertDialogContent(class: 'rounded-[2rem] border-none shadow-2xl') do
            AlertDialogHeader do
              AlertDialogTitle { 'Delete Location' }
              AlertDialogDescription do
                "Are you sure you want to delete #{location.name}? " \
                  'This will also delete all medicines at this location.'
              end
            end
            AlertDialogFooter do
              AlertDialogCancel(class: 'rounded-xl') { 'Cancel' }
              form_with(url: location_path(location), method: :delete, class: 'inline') do
                Button(variant: :destructive, type: :submit, class: 'rounded-xl shadow-lg shadow-destructive/20') do
                  'Delete'
                end
              end
            end
          end
        end
      end
    end
  end
end
