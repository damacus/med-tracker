# frozen_string_literal: true

module Api
  module V1
    module Sync
      class FeedsController < Api::V1::BaseController
        def snapshot
          render json: { data: Api::SyncSnapshot.new(household: current_household, exporter: exporter).payload }
        end

        def changes
          since = Time.iso8601(params.expect(:cursor))
          render json: { data: changes_payload(since) }
        rescue ArgumentError
          render_unprocessable('cursor must be ISO8601')
        end

        private

        def exporter
          PortableData::Exporter.new(
            household: current_household,
            membership: current_membership,
            passphrase: nil,
            request: request
          )
        end

        def changes_payload(since)
          Api::ConsistentSyncRead.new(household: current_household).call do |cursor|
            {
              cursor: cursor,
              changes: change_events_since(since).map { |event| change_payload(event) },
              tombstones: tombstones_since(since).map { |tombstone| tombstone_payload(tombstone) }
            }
          end
        end

        def change_events_since(since)
          ApiChangeEvent.where(household: current_household).where(occurred_at: since..).order(:occurred_at, :id)
        end

        def tombstones_since(since)
          ApiTombstone.where(household: current_household).where(deleted_at: since..).order(:deleted_at, :id)
        end

        def change_payload(event)
          {
            id: event.id,
            record_type: event.record_type,
            record_id: event.record_id,
            record_portable_id: event.record_portable_id,
            action: event.action,
            occurred_at: event.occurred_at.iso8601,
            metadata: event.metadata
          }
        end

        def tombstone_payload(tombstone)
          {
            id: tombstone.id,
            record_type: tombstone.record_type,
            record_portable_id: tombstone.record_portable_id,
            action: tombstone.action,
            deleted_at: tombstone.deleted_at.iso8601,
            metadata: tombstone.metadata
          }
        end
      end
    end
  end
end
