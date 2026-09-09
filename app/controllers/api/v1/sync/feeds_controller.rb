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
            request: request,
            people_scope: policy_scope(Person)
          )
        end

        def changes_payload(since)
          Api::ConsistentSyncRead.new(household: current_household).call do |cursor|
            events = outcome_projection.events(change_events_since(since)).to_a
            @outcome_payloads = outcome_projection.payloads(events)
            {
              cursor: cursor,
              changes: events.filter_map { |event| change_payload(event) },
              tombstones: outcome_projection.tombstones(tombstones_since(since)).map { |row| tombstone_payload(row) }
            }
          end
        end

        def outcome_projection
          @outcome_projection ||= Api::DoseOutcomeSyncProjection.new(household: current_household,
                                                                     people_scope: policy_scope(Person))
        end

        def change_events_since(since)
          scope = ApiChangeEvent.where(household: current_household).where(occurred_at: since..)
          ordinary_events = scope.where.not(record_type: 'MedicationPausePeriod')
          pause_events = scope.where(record_type: 'MedicationPausePeriod', record_id: visible_pause_period_ids)
          ordinary_events.or(pause_events).order(:occurred_at, :id)
        end

        def tombstones_since(since)
          ApiTombstone.where(household: current_household).where.not(record_type: 'MedicationPausePeriod')
                      .where(deleted_at: since..).order(:deleted_at, :id)
        end

        def visible_pause_period_ids
          Api::MedicationPausePeriodVisibility.new(
            household: current_household, person_scope: policy_scope(Person)
          ).periods.select(:id)
        end

        def change_payload(event)
          if event.record_type == Api::DoseOutcomeSyncProjection::RECORD_TYPE &&
             !@outcome_payloads.key?(event.record_portable_id)
            return
          end

          {
            id: event.id,
            record_type: event.record_type,
            record_id: event.record_id,
            record_portable_id: event.record_portable_id,
            action: event.action,
            occurred_at: event.occurred_at.iso8601,
            metadata: event.metadata
          }.merge(outcome_record_payload(event))
        end

        def outcome_record_payload(event)
          return {} unless event.record_type == Api::DoseOutcomeSyncProjection::RECORD_TYPE

          { record: @outcome_payloads.fetch(event.record_portable_id) }
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
