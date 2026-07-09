# frozen_string_literal: true

module Reports
  class MedicationReviewPdf
    module OpeningSections
      LEGEND = [
        ['HIGH', 'Source evidence describes a serious risk'],
        ['MODERATE', 'Source evidence warrants review'],
        ['LOW', 'Lower source risk; filtered by default'],
        ['UNKNOWN', 'Not classified; not the highest risk']
      ].freeze

      private

      def render_header
        top = document.cursor
        render_header_background(top)
        render_header_copy(top)
        document.move_down 132
        document.fill_color COLORS.fetch(:ink)
      end

      def render_header_background(top)
        document.fill_color COLORS.fetch(:forest)
        document.fill_rectangle [0, top], document.bounds.width, 112
      end

      def render_header_copy(top)
        document.fill_color COLORS.fetch(:white)
        document.text_box 'MEDTRACKER', at: [22, top - 22], size: 9, style: :bold, character_spacing: 1.4
        document.text_box 'Medicine review record', at: [22, top - 45], size: 24, style: :bold
        document.text_box generated_label, at: [22, top - 82], size: 9
      end

      def render_summary
        document.table([summary_data], width: document.bounds.width,
                                       cell_style: { borders: [], padding: [9, 12] }) do |table|
          style_summary(table)
        end
        document.move_down 16
      end

      def summary_data
        [
          summary_cell('TO DISCUSS', prompts.count { |prompt| prompt.status == 'needs_review' }),
          summary_cell('REVIEWED', prompts.count(&:practitioner_review_status?)),
          summary_cell('PEOPLE', prompts.map(&:person_id).uniq.size)
        ]
      end

      def style_summary(table)
        table.cells.background_color = COLORS.fetch(:paper)
        table.columns(0..2).width = document.bounds.width / 3
      end

      def summary_cell(label, value)
        "#{value}\n#{label}"
      end

      def render_boundary
        document.table([[boundary_text]], width: document.bounds.width, cell_style: { padding: 12 }) do |table|
          style_boundary(table)
        end
        document.move_down 16
      end

      def style_boundary(table)
        table.cells.background_color = COLORS.fetch(:mint)
        table.cells.border_color = COLORS.fetch(:forest)
        table.cells.text_color = COLORS.fetch(:ink)
        table.cells.size = 9
      end

      def render_legend
        render_legend_heading
        document.table(legend_rows, width: document.bounds.width,
                                    cell_style: { borders: [], padding: [3, 8, 3, 0], size: 8 }) do |table|
          style_legend(table)
        end
        document.move_down 18
      end

      def render_legend_heading
        document.text 'RISK LEVELS IN THIS RECORD', size: 8, style: :bold, color: COLORS.fetch(:muted),
                                                    character_spacing: 0.8
        document.move_down 6
      end

      def legend_rows
        LEGEND.each_slice(2).map(&:flatten)
      end

      def style_legend(table)
        style_legend_column(table.columns(0), COLORS.fetch(:coral))
        style_legend_column(table.columns(2), COLORS.fetch(:amber))
      end

      def style_legend_column(column, color)
        column.font_style = :bold
        column.text_color = color
      end
    end
  end
end
