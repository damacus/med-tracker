# frozen_string_literal: true

module Components
  module Offline
    class Shell < Components::Base
      def view_template
        div(
          data: {
            controller: 'offline-shell',
            offline_shell_snapshot_url_value: '/offline/snapshot',
            offline_shell_sync_url_value: '/offline/medication_takes'
          },
          class: 'min-h-screen bg-background text-foreground'
        ) do
          div(class: 'container mx-auto max-w-6xl px-4 py-8 space-y-6') do
            render_header
            render_status_bar
            render_content
          end
        end
      end

      private

      def render_header
        header(class: 'flex flex-col gap-4 border-b border-border pb-6 md:flex-row md:items-end md:justify-between') do
          div do
            m3_text(size: '2', weight: 'muted', class: 'uppercase font-bold tracking-widest') { 'Offline care' }
            m3_heading(level: 1, size: '7', class: 'font-bold tracking-tight') { 'Medication dashboard' }
          end
          m3_button(
            type: :button,
            variant: :filled,
            class: 'rounded-xl',
            data: { action: 'offline-shell#refreshSnapshot' }
          ) { 'Refresh' }
        end
      end

      def render_status_bar
        section(class: 'grid gap-3 md:grid-cols-4', data: { offline_shell_target: 'status' }) do
          render_status_card('Connection', 'Checking...', 'connection')
          render_status_card('Snapshot', 'Not cached', 'snapshotAge')
          render_status_card('Pending sync', '0', 'pendingCount')
          render_status_card('Needs attention', '0', 'failedCount')
        end
      end

      def render_status_card(label, value, target)
        div(class: 'rounded-lg border border-border bg-surface-container-low p-4') do
          p(class: 'text-xs font-bold uppercase tracking-widest text-on-surface-variant') { label }
          p(class: 'mt-2 text-lg font-bold', data: { offline_shell_target: target }) { value }
        end
      end

      def render_content
        main(class: 'grid gap-6 lg:grid-cols-[1.2fr_0.8fr]') do
          section(class: 'space-y-4') do
            m3_heading(level: 2, size: '5', class: 'font-bold') { 'Today' }
            div(data: { offline_shell_target: 'today' }, class: 'space-y-3') do
              render_empty_state('Open the app while online to cache your care plan.')
            end
          end

          aside(class: 'space-y-4') do
            m3_heading(level: 2, size: '5', class: 'font-bold') { 'People and inventory' }
            div(data: { offline_shell_target: 'people' }, class: 'space-y-3')
            div(data: { offline_shell_target: 'failures' }, class: 'space-y-3')
          end
        end
      end

      def render_empty_state(message)
        div(class: 'rounded-lg border border-dashed border-border bg-surface-container-low p-8 text-center') do
          p(class: 'text-sm text-on-surface-variant') { message }
        end
      end
    end
  end
end
