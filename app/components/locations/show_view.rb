# frozen_string_literal: true

module Components
  module Locations
    class ShowView < Components::Base
      attr_reader :location, :notice, :available_people

      def initialize(location:, notice: nil, available_people: [])
        @location = location
        @notice = notice
        @available_people = available_people
        super()
      end

      def view_template
        div(
          id: tenant_dom_target("location_show_#{location.id}"),
          class: 'container mx-auto px-4 py-6 md:py-10 max-w-6xl space-y-6'
        ) do
          render_notice if notice.present?
          render_header

          div(class: 'grid grid-cols-1 lg:grid-cols-3 gap-6') do
            div(class: 'lg:col-span-2 space-y-8') do
              render_medications_section
            end

            div(class: 'space-y-8') do
              render_members_card
              render_details_card
            end
          end
        end
      end

      private

      def render_notice
        render RubyUI::Alert.new(variant: :success, class: 'mb-8 rounded-2xl border-none shadow-sm') do
          plain(notice)
        end
      end

      def render_header
        m3_card(variant: :filled,
                class: 'p-5 md:p-8 flex flex-col md:flex-row md:items-center justify-between gap-6') do
          div(class: 'flex items-center gap-4 min-w-0') do
            div(
              class: 'w-14 h-14 shrink-0 rounded-shape-lg bg-primary-container flex items-center ' \
                     'justify-center text-on-primary-container'
            ) do
              render Icons::Home.new(size: 32)
            end
            div(class: 'space-y-1 min-w-0') do
              m3_text(variant: :label_medium, class: 'text-on-surface-variant') do
                t('locations.show.location')
              end
              m3_heading(level: 1, variant: :headline_large, class: 'font-bold break-words') { location.name }
            end
          end

          div(class: 'flex flex-wrap gap-2 shrink-0') do
            if view_context.policy(location).update?
              m3_link(href: edit_location_path(location, return_to: location_path(location)),
                      variant: :filled, size: :lg) do
                t('locations.show.edit_location')
              end
            end
            m3_link(href: locations_path, variant: :text, size: :lg) do
              t('locations.show.all_locations')
            end
          end
        end
      end

      def render_medications_section
        div(class: 'space-y-4') do
          div(class: 'flex items-center justify-between') do
            m3_heading(level: 2, size: '5', class: 'font-bold tracking-tight') do
              t('locations.show.medications_heading')
            end
          end

          if location.medications.present?
            div(class: 'grid grid-cols-1 md:grid-cols-2 gap-4') do
              location.medications.each do |medication|
                render_medication_card(medication)
              end
            end
          else
            m3_card(class: 'p-8 text-center') do
              m3_text(size: '3', class: 'text-on-surface-variant') { t('locations.show.no_medications') }
            end
          end
        end
      end

      def render_medication_card(medication)
        m3_card(variant: :outlined, class: 'p-4 bg-surface-container-lowest border-outline-variant overflow-hidden') do
          div(class: 'flex flex-col items-start gap-3') do
            div(class: 'flex items-start gap-4 min-w-0 flex-1') do
              div(
                class: 'w-10 h-10 rounded-xl bg-secondary-container flex items-center ' \
                       'justify-center text-on-surface-variant flex-shrink-0'
              ) do
                render Components::Shared::MedicationIcon.new(medication: medication, size: 20)
              end
              div(class: 'min-w-0 flex-1') do
                m3_link(
                  href: medication_path(medication),
                  variant: :link,
                  class: 'h-auto p-0 font-semibold text-base no-underline whitespace-normal break-words ' \
                         'text-left leading-snug text-on-surface'
                ) do
                  medication.display_name
                end
                if medication.dose_amount.present? && medication.dose_unit.present?
                  m3_text(size: '1', class: 'text-on-surface-variant') do
                    DoseAmount.new(medication.dose_amount, medication.dose_unit).to_s
                  end
                end
              end
            end
          end

          div(class: 'flex flex-wrap items-center justify-between gap-3 mt-3 pt-3 border-t border-outline-variant') do
            render_stock_badges(medication)
            if view_context.policy(medication).refill?
              render Components::Medications::RefillModal.new(
                medication: medication,
                button_variant: :outlined,
                button_class: 'shrink-0'
              )
            end
          end
        end
      end

      def render_stock_badges(medication)
        div(class: 'flex flex-wrap items-center gap-2') do
          m3_badge(variant: :tonal, class: 'shrink-0 whitespace-nowrap') do
            ::Medications::SupplyStatusPresenter.new(medication: medication).inventory_units_label
          end
          Badge(variant: :destructive, class: 'shrink-0 whitespace-nowrap') { 'Low Stock' } if medication.low_stock?
        end
      end

      def render_members_card
        m3_card(variant: :filled, class: 'p-5 md:p-6 space-y-6') do
          div(class: 'flex items-center justify-between') do
            m3_heading(level: 3, size: '4', class: 'font-bold') { t('locations.show.members') }
            render_add_member_dialog if view_context.policy(location).update?
          end

          if location.members.present?
            div(class: 'space-y-3') do
              # Bolt: Pre-index memberships by person_id to convert O(N^2) search to O(N) with O(1) hash lookups
              memberships_by_person_id = location.location_memberships.index_by(&:person_id)
              location.members.each do |member|
                div(class: 'flex items-center justify-between group') do
                  div(class: 'flex items-center gap-3') do
                    render Components::Shared::PersonAvatar.new(person: member, size: :sm)
                    m3_text(size: '2', weight: 'semibold', class: 'text-foreground') { member.name }
                  end

                  if view_context.policy(location).update?
                    membership = memberships_by_person_id[member.id]
                    render_remove_member_dialog(member, membership)
                  end
                end
              end
            end
          else
            m3_text(size: '2', class: 'text-on-surface-variant italic') { t('locations.show.no_members') }
          end
        end
      end

      def render_remove_member_dialog(member, membership)
        AlertDialog do
          AlertDialogTrigger do
            m3_button(
              variant: :text,
              size: :lg,
              icon: true,
              class: 'opacity-0 group-hover:opacity-100 transition-opacity text-on-surface-variant ' \
                     'hover:text-destructive',
              aria_label: t('locations.show.remove_member.aria_label', default: 'Remove member')
            ) do
              render Icons::X.new(size: 14, aria_hidden: 'true')
            end
          end

          AlertDialogContent(class: 'rounded-[2rem] border-none shadow-2xl') do
            AlertDialogHeader do
              AlertDialogTitle { t('locations.show.remove_member.title') }
              AlertDialogDescription do
                t('locations.show.remove_member.confirm', name: member.name, location: location.name)
              end
            end

            AlertDialogFooter do
              AlertDialogCancel { t('locations.show.remove_member.cancel') }
              form_with(url: location_location_membership_path(location, membership), method: :delete,
                        class: 'inline') do
                m3_button(variant: :destructive, type: :submit, class: 'shadow-elevation-2') do
                  t('locations.show.remove_member.submit')
                end
              end
            end
          end
        end
      end

      def render_details_card
        m3_card(variant: :filled, class: 'p-5 md:p-6 space-y-4') do
          div(class: 'flex items-center justify-between') do
            m3_heading(level: 3, size: '4', class: 'font-bold') { t('locations.show.details') }
            if view_context.policy(location).update?
              Link(
                href: edit_location_path(location, return_to: location_path(location)),
                variant: :text,
                size: :sm,
                class: 'text-on-surface-variant hover:text-primary h-8 w-8 p-0 flex items-center justify-center',
                aria_label: t('locations.show.edit_details', default: 'Edit location details')
              ) do
                render Icons::Pencil.new(size: 16, aria_hidden: 'true')
              end
            end
          end

          if location.description.present?
            m3_text(size: '2', class: 'text-on-surface-variant leading-relaxed') { location.description }
          else
            m3_text(size: '2', class: 'text-on-surface-variant italic') { t('locations.show.no_details') }
          end
        end
      end

      def render_add_member_dialog
        Dialog do
          DialogTrigger do
            m3_button(
              variant: :text,
              size: :sm,
              class: 'w-8 h-8 p-0 rounded-full bg-secondary-container text-on-surface-variant ' \
                     'hover:text-primary hover:bg-primary/5',
              aria_label: t('locations.show.add_member.aria_label', default: 'Add member')
            ) do
              render Icons::Plus.new(size: 16, aria_hidden: 'true')
            end
          end

          DialogContent(size: :md) do
            DialogHeader do
              DialogTitle { t('locations.show.add_member.title') }
              DialogDescription { t('locations.show.add_member.description', name: location.name) }
            end

            DialogMiddle do
              if available_people.any?
                form_with(url: location_location_memberships_path(location), method: :post, class: 'space-y-4') do
                  div(class: 'space-y-2') do
                    label(for: 'location_membership_person_id', class: 'text-sm font-medium') do
                      t('locations.show.add_member.select_person')
                    end
                    m3_select(
                      name: 'location_membership[person_id]',
                      id: 'location_membership_person_id',
                      size: :sm,
                      required: true
                    ) do
                      option(value: '') { t('locations.show.add_member.placeholder') }
                      available_people.each do |person|
                        option(value: person.id) { person.name }
                      end
                    end
                  end

                  div(class: 'flex justify-end gap-3 pt-2') do
                    m3_button(type: :submit, variant: :filled) { t('locations.show.add_member.submit') }
                  end
                end
              else
                div(class: 'py-8 text-center space-y-2') do
                  div(
                    class: 'w-12 h-12 rounded-full bg-secondary-container flex items-center justify-center ' \
                           'text-on-surface-variant mx-auto'
                  ) do
                    render Icons::Users.new(size: 24)
                  end
                  m3_text(size: '2', class: 'text-on-surface-variant font-medium') do
                    t('locations.show.add_member.all_assigned')
                  end
                end
              end
            end
          end
        end
      end
    end
  end
end
