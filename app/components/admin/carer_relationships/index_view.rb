# frozen_string_literal: true

module Components
  module Admin
    module CarerRelationships
      class IndexView < Components::Base
        include Phlex::Rails::Helpers::ButtonTo
        include Phlex::Rails::Helpers::FormWith

        attr_reader :relationships, :current_user, :pagy_obj

        def initialize(relationships:, current_user: nil, pagy: nil)
          @relationships = relationships
          @current_user = current_user
          @pagy_obj = pagy
          super()
        end

        def view_template
          div(data: { testid: 'admin-carer-relationships' },
              class: 'container mx-auto px-4 py-8 pb-24 md:pb-8 max-w-6xl space-y-8') do
            render_header
            render_relationships_table
            render_pagination if pagy_obj && pagy_obj.pages > 1
          end
        end

        private

        def render_header
          div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 mb-12') do
            div do
              m3_text(size: '2', weight: 'muted', class: 'uppercase tracking-widest mb-1 block font-bold') do
                Time.current.strftime('%A, %b %d')
              end
              m3_heading(level: 1, size: '8', class: 'font-extrabold tracking-tight') do
                t('admin.carer_relationships.index.title')
              end
              m3_text(weight: 'muted', class: 'mt-2 block') { t('admin.carer_relationships.index.subtitle') }
            end
            render RubyUI::Link.new(
              href: '/admin/carer_relationships/new',
              variant: :filled,
              size: :lg,
              class: 'rounded-2xl shadow-lg shadow-primary/20',
              data: { turbo_frame: 'modal' }
            ) { t('admin.carer_relationships.index.new_relationship') }
          end
        end

        def render_relationships_table
          div(class: 'rounded-xl border border-border bg-card shadow-sm overflow-hidden') do
            render RubyUI::Table.new do
              render_table_header
              render_table_body
            end
          end
        end

        def render_table_header
          render RubyUI::TableHeader.new do
            render RubyUI::TableRow.new do
              render(RubyUI::TableHead.new { t('admin.carer_relationships.index.table.carer') })
              render(RubyUI::TableHead.new { t('admin.carer_relationships.index.table.patient') })
              render(RubyUI::TableHead.new { t('admin.carer_relationships.index.table.type') })
              render(RubyUI::TableHead.new { t('admin.carer_relationships.index.table.status') })
              render RubyUI::TableHead.new(class: 'text-right') { t('admin.carer_relationships.index.table.actions') }
            end
          end
        end

        def render_table_body
          render RubyUI::TableBody.new(id: 'carer_relationships_rows') do
            if relationships.empty?
              render RubyUI::TableRow.new(id: 'carer_relationships_empty') do
                render RubyUI::TableCell.new(colspan: 5, class: 'py-8 text-center text-on-surface-variant') do
                  t('admin.carer_relationships.index.empty')
                end
              end
            else
              relationships.each do |relationship|
                render_relationship_row(relationship)
              end
            end
          end
        end

        def render_relationship_row(relationship)
          render Row.new(relationship: relationship)
        end

        def render_status_badge(relationship)
          if relationship.active?
            render RubyUI::Badge.new(variant: :success) { t('admin.carer_relationships.index.active') }
          else
            render RubyUI::Badge.new(variant: :destructive) { t('admin.carer_relationships.index.inactive') }
          end
        end

        def render_activation_button(relationship)
          if relationship.active?
            render_deactivate_dialog(relationship)
          else
            form_with(
              url: "/admin/carer_relationships/#{relationship.id}/activate",
              method: :post,
              class: 'inline-block'
            ) do
              m3_button(
                type: :submit,
                variant: :success_outline,
                size: :sm
              ) { t('admin.carer_relationships.index.activate') }
            end
          end
        end

        def render_deactivate_dialog(relationship)
          render RubyUI::AlertDialog.new do
            render RubyUI::AlertDialogTrigger.new do
              m3_button(variant: :destructive_outline, size: :sm) do
                t('admin.carer_relationships.index.deactivate')
              end
            end
            render RubyUI::AlertDialogContent.new do
              render RubyUI::AlertDialogHeader.new do
                render(RubyUI::AlertDialogTitle.new { t('admin.carer_relationships.index.deactivate_dialog.title') })
                render RubyUI::AlertDialogDescription.new do
                  t('admin.carer_relationships.index.deactivate_dialog.confirm',
                    carer: relationship.carer.name,
                    patient: relationship.patient.name)
                end
              end
              render RubyUI::AlertDialogFooter.new do
                render(RubyUI::AlertDialogCancel.new { t('admin.carer_relationships.index.deactivate_dialog.cancel') })
                form_with(url: "/admin/carer_relationships/#{relationship.id}", method: :delete, class: 'inline') do
                  m3_button(variant: :destructive, type: :submit) do
                    t('admin.carer_relationships.index.deactivate_dialog.submit')
                  end
                end
              end
            end
          end
        end

        def render_pagination
          div(
            class: 'flex items-center justify-between border-t border-border ' \
                   'bg-card px-4 py-3 sm:px-6'
          ) do
            div(data: { testid: 'pagination-info' }) do
              m3_text(size: '2', class: 'text-foreground') do
                plain "#{t('admin.carer_relationships.index.pagination.showing')} "
                span(class: 'font-medium') { pagy_obj.from.to_s }
                plain " #{t('admin.carer_relationships.index.pagination.to')} "
                span(class: 'font-medium') { pagy_obj.to.to_s }
                plain " #{t('admin.carer_relationships.index.pagination.of')} "
                span(class: 'font-medium') { pagy_obj.count.to_s }
                plain " #{t('admin.carer_relationships.index.pagination.results')}"
              end
            end
          end
        end
      end
    end
  end
end