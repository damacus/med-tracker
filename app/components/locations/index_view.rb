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
        div(id: 'locations_index', class: 'container mx-auto px-4 py-12 max-w-6xl',
            data: { testid: 'locations-list' }) do
          render_header
          render_locations_grid
        end
      end

      private

      def render_header
        header(class: 'mb-10 flex flex-col gap-6 md:flex-row md:items-end md:justify-between') do
          div do
            m3_text(size: '2', weight: 'muted', class: 'uppercase tracking-widest mb-1 block font-bold') do
              t('locations.index.manage_locations')
            end
            m3_heading(level: 1, size: '7', class: 'font-bold tracking-tight') { t('locations.index.title') }
          end
          if view_context.policy(Location).create?
            m3_link(
              href: new_location_path,
              variant: :filled,
              size: :lg,
              class: 'w-full justify-center font-bold text-sm shadow-elevation-2 md:w-auto'
            ) do
              span { t('locations.index.add_location') }
            end
          end
        end
      end

      def render_locations_grid
        div(class: 'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8', id: tenant_dom_target('locations')) do
          locations.each do |location|
            render_location_card(location)
          end
        end
      end

      def render_location_card(location)
        m3_card(
          id: tenant_dom_id(location),
          variant: :elevated,
          class: 'group h-full overflow-hidden bg-surface-container-low transition-all duration-300 ' \
                 'hover:shadow-elevation-2'
        ) do
          m3_card_header(class: 'px-8 pb-4 pt-8') do
            div(class: 'mb-4 flex items-start justify-between') do
              render_location_icon
              m3_badge(variant: :outlined, class: 'border-outline-variant text-on-surface-variant') do
                pluralize(location.medications.size, t('medications.created').split.first.downcase)
              end
            end
            m3_heading(variant: :title_large, level: 2, class: 'font-bold tracking-tight') { location.name }
          end

          m3_card_content(class: 'flex-grow space-y-4 px-8 pb-4') do
            if location.description.present?
              m3_text(variant: :body_medium, class: 'line-clamp-2 leading-relaxed text-on-surface-variant') do
                location.description
              end
            end

            if location.members.present?
              div(class: 'border-t border-outline-variant/30 pt-4') do
                m3_text(variant: :label_small,
                        class: 'mb-2 block font-bold uppercase tracking-widest text-on-surface-variant') do
                  t('locations.index.members')
                end
                div(class: 'flex flex-wrap gap-1') do
                  location.members.each do |member|
                    m3_badge(
                      variant: :outlined,
                      class: 'border-outline-variant bg-secondary-container text-on-secondary-container'
                    ) { member.name }
                  end
                end
              end
            end
          end

          m3_card_footer(class: 'mt-auto px-8 pb-8 pt-2') do
            render_location_actions(location)
          end
        end
      end

      def render_location_icon
        div(
          class: 'flex h-12 w-12 items-center justify-center rounded-shape-lg bg-secondary-container ' \
                 'text-on-secondary-container transition-colors group-hover:bg-primary-container ' \
                 'group-hover:text-on-primary-container'
        ) do
          render Icons::Home.new(size: 24)
        end
      end

      def render_location_actions(location)
        location_policy = view_context.policy(location)

        div(class: 'flex items-center gap-2 w-full') do
          m3_link(
            href: location_path(location),
            variant: :outlined,
            size: :sm,
            class: 'flex-1 border-outline bg-surface-container-low text-on-surface ' \
                   'hover:bg-surface-container-high'
          ) do
            t('locations.index.view')
          end
          if location_policy.update?
            m3_link(
              href: edit_location_path(location, return_to: locations_path),
              variant: :outlined,
              size: :lg,
              icon: true,
              class: 'border-outline bg-surface-container-low text-on-surface ' \
                     'hover:bg-surface-container-high',
              aria_label: t('locations.index.edit', default: 'Edit location')
            ) do
              render Icons::Pencil.new(size: 16, aria_hidden: 'true')
            end
          end
          render_delete_dialog(location) if location_policy.destroy?
        end
      end

      def render_delete_dialog(location)
        AlertDialog do
          AlertDialogTrigger do
            m3_button(variant: :text, size: :lg, icon: true,
                      class: 'text-on-surface-variant ' \
                             'hover:text-destructive hover:bg-destructive/5',
                      aria_label: t('locations.index.delete', default: 'Delete location')) do
              render Icons::Trash.new(size: 18, aria_hidden: 'true')
            end
          end
          AlertDialogContent(class: 'rounded-shape-xl border-outline bg-surface-container-high shadow-elevation-3') do
            AlertDialogHeader do
              AlertDialogTitle { t('locations.index.delete_dialog.title') }
              AlertDialogDescription do
                t('locations.index.delete_dialog.confirm', name: location.name)
              end
            end
            AlertDialogFooter do
              AlertDialogCancel { t('locations.index.delete_dialog.cancel') }
              form_with(url: location_path(location), method: :delete, class: 'inline') do
                m3_button(variant: :destructive, type: :submit, class: 'shadow-elevation-2') do
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
