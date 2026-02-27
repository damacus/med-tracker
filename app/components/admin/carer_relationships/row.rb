# frozen_string_literal: true

module Components
  module Admin
    module CarerRelationships
      class Row < Components::Base
        include Phlex::Rails::Helpers::FormWith

        attr_reader :relationship

        def initialize(relationship:)
          @relationship = relationship
          super()
        end

        def view_template
          row_class = relationship.active? ? '' : 'opacity-60'
          render RubyUI::TableRow.new(
            id: "carer_relationship_#{relationship.id}",
            class: row_class,
            data: { relationship_id: relationship.id }
          ) do
            render(RubyUI::TableCell.new(class: 'font-medium') { relationship.carer.name })
            render(RubyUI::TableCell.new { relationship.patient.name })
            render(RubyUI::TableCell.new(class: 'capitalize') { relationship.relationship_type.to_s.humanize })
            render(RubyUI::TableCell.new { render_status_badge })
            render RubyUI::TableCell.new(class: 'text-right space-x-2') do
              render_activation_button
            end
          end
        end

        private

        def render_status_badge
          if relationship.active?
            render RubyUI::Badge.new(variant: :green) { t('admin.carer_relationships.index.active') }
          else
            render RubyUI::Badge.new(variant: :red) { t('admin.carer_relationships.index.inactive') }
          end
        end

        def render_activation_button
          if relationship.active?
            render_deactivate_dialog
          else
            form_with(
              url: "/admin/carer_relationships/#{relationship.id}/activate",
              method: :post,
              class: 'inline-block'
            ) do
              Button(
                type: :submit,
                variant: :success_outline,
                size: :sm
              ) { t('admin.carer_relationships.index.activate') }
            end
          end
        end

        def render_deactivate_dialog
          render RubyUI::AlertDialog.new do
            render RubyUI::AlertDialogTrigger.new do
              Button(variant: :destructive_outline, size: :sm) do
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
                  Button(variant: :destructive, type: :submit) do
                    t('admin.carer_relationships.index.deactivate_dialog.submit')
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
