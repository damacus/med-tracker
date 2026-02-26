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
              t('locations.index.manage_locations')
            end
            Heading(level: 1, size: '8', class: 'font-extrabold tracking-tight') { t('locations.index.title') }
          end
          Link(
            href: new_location_path,
            variant: :primary,
            size: :lg,
            class: 'rounded-2xl font-bold text-sm shadow-lg shadow-primary/20'
          ) do
            span { t('locations.index.add_location') }
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
              # Fallback for medication word
              Badge(variant: :outline) do
                pluralize(location.medications.size, t('medications.created').split.first.downcase)
              end
            end
            Heading(level: 2, size: '5', class: 'font-bold tracking-tight') { location.name }
          end

          CardContent(class: 'flex-grow space-y-4 px-8 pb-4') do
            if location.description.present?
              Text(size: '2', class: 'text-slate-400 line-clamp-2 leading-relaxed') { location.description }
            end

            if location.members.any?
              div(class: 'pt-4 border-t border-slate-100') do
                Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-slate-500 mb-2 block') do
                  t('locations.index.members')
                end
                div(class: 'flex flex-wrap gap-1') do
                  location.members.each do |member|
                    Badge(variant: :outline,
                          class: 'text-[10px] bg-slate-50/50 text-slate-600 border-slate-200 font-medium') do
                      member.name
                    end
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
            t('locations.index.view')
          end
          Link(
            href: edit_location_path(location, return_to: locations_path),
            variant: :outline,
            size: :sm,
            class: 'rounded-xl w-10 h-10 p-0 border-slate-100 bg-white hover:bg-slate-50 text-slate-400'
          ) do
            render Icons::Pencil.new(size: 16)
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
              AlertDialogTitle { t('locations.index.delete_dialog.title') }
              AlertDialogDescription do
                t('locations.index.delete_dialog.confirm', name: location.name)
              end
            end
            AlertDialogFooter do
              AlertDialogCancel(class: 'rounded-xl') { t('locations.index.delete_dialog.cancel') }
              form_with(url: location_path(location), method: :delete, class: 'inline') do
                Button(variant: :destructive, type: :submit, class: 'rounded-xl shadow-lg shadow-destructive/20') do
                  t('locations.index.delete_dialog.submit')
                end
              end
            end
          end
        end
      end
    end
  end
end
