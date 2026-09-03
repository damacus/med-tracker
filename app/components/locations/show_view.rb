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
          class: 'container mx-auto px-4 py-12 max-w-6xl space-y-12'
        ) do
          render_notice if notice.present?
          render_header

          div(class: 'grid grid-cols-1 lg:grid-cols-3 gap-12') do
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
        render RubyUI::Alert.new(
          variant: :success,
          class: 'mb-8 rounded-shape-xl border-outline bg-success-container ' \
                 'text-on-success-container shadow-elevation-1'
        ) do
          plain(notice)
        end
      end

      def render_header
        div(
          class: 'flex flex-col justify-between gap-6 border-b border-outline-variant/30 pb-8 ' \
                 'md:flex-row md:items-end'
        ) do
          div(class: 'flex items-center gap-6') do
            div(
              class: 'flex h-20 w-20 items-center justify-center rounded-shape-xl bg-primary-container ' \
                     'text-on-primary-container shadow-elevation-1'
            ) do
              render Icons::Home.new(size: 32)
            end
            div(class: 'space-y-1') do
              m3_text(size: '2', weight: 'bold',
                      class: 'uppercase tracking-[0.2em] font-black opacity-40 block mb-1') do
                t('locations.show.location')
              end
              m3_heading(level: 1, size: '8', class: 'font-black tracking-tight') { location.name }
            end
          end

          div(class: 'flex gap-3') do
            if view_context.policy(location).update?
              m3_link(
                href: edit_location_path(location, return_to: location_path(location)),
                variant: :outlined,
                size: :lg,
                class: 'border-outline bg-surface-container-low font-bold text-sm text-on-surface'
              ) do
                t('locations.show.edit_location')
              end
            end
            m3_link(
              href: locations_path,
              variant: :text,
              size: :lg,
              class: 'font-bold text-sm text-on-surface-variant hover:text-on-surface'
            ) do
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
            m3_card(variant: :outlined, class: 'border-dashed border-outline-variant p-8 text-center') do
              m3_text(size: '3', class: 'text-on-surface-variant') { t('locations.show.no_medications') }
            end
          end
        end
      end

      def render_medication_card(medication)
        m3_card(
          variant: :elevated,
          class: 'overflow-hidden p-6 transition-shadow hover:shadow-elevation-2'
        ) do
          div(class: 'flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between') do
            div(class: 'flex items-start gap-4 min-w-0 flex-1') do
              div(
                class: 'flex h-10 w-10 shrink-0 items-center justify-center rounded-shape-md bg-secondary-container ' \
                       'text-on-secondary-container'
              ) do
                render Components::Shared::MedicationIcon.new(medication: medication, size: 20)
              end
              div(class: 'min-w-0 flex-1') do
                m3_link(
                  href: medication_path(medication),
                  variant: :text,
                  class: 'h-auto p-0 text-left text-base font-semibold leading-snug no-underline whitespace-normal ' \
                         'break-words text-on-surface hover:text-primary'
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
            if medication.low_stock?
              m3_badge(variant: :destructive, class: 'shrink-0 justify-center whitespace-nowrap') { 'Low Stock' }
            else
              m3_badge(variant: :success, class: 'shrink-0 justify-center whitespace-nowrap') do
                ::Medications::SupplyStatusPresenter.new(medication: medication).inventory_units_label
              end
            end
          end

          if view_context.policy(medication).refill?
            div(class: 'pt-4') do
              render Components::Medications::RefillModal.new(
                medication: medication,
                button_variant: :outlined,
                button_class: 'w-full'
              )
            end
          end
        end
      end

      def render_members_card
        m3_card(variant: :elevated, class: 'space-y-6 p-8') do
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

          AlertDialogContent(class: 'rounded-shape-xl border-outline bg-surface-container-high shadow-elevation-3') do
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
        m3_card(variant: :filled, class: 'space-y-4 p-8') do
          div(class: 'flex items-center justify-between') do
            m3_heading(level: 3, size: '4', class: 'font-bold') { t('locations.show.details') }
            if view_context.policy(location).update?
              m3_link(
                href: edit_location_path(location, return_to: location_path(location)),
                variant: :text,
                size: :sm,
                class: 'flex h-8 w-8 items-center justify-center p-0 text-on-surface-variant hover:text-primary',
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
              class: 'h-8 w-8 rounded-shape-full bg-secondary-container p-0 text-on-secondary-container ' \
                     'hover:bg-primary-container hover:text-on-primary-container',
              aria_label: t('locations.show.add_member.aria_label', default: 'Add member')
            ) do
              render Icons::Plus.new(size: 16, aria_hidden: 'true')
            end
          end

          DialogContent(
            size: :md,
            class: 'rounded-shape-xl border-outline bg-surface-container-high shadow-elevation-3'
          ) do
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
                    class: 'mx-auto flex h-12 w-12 items-center justify-center rounded-shape-full ' \
                           'bg-secondary-container text-on-secondary-container'
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
