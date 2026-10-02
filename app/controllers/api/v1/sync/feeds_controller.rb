# frozen_string_literal: true

module Api
  module V1
    module Sync
      class FeedsController < Api::V1::BaseController
        PERSON_SCOPED_RECORD_TYPES = %w[
          HealthEvent MedicationDoseOccurrence MedicationPausePeriod MedicationTake NotificationPreference Person
          PersonMedication Schedule
        ].freeze
        HOUSEHOLD_WIDE_RECORD_TYPES = %w[
          Location MedicationDosageOption
        ].freeze

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
          ordinary_events = scope.where.not(record_type: %w[Medication MedicationPausePeriod])
          ordinary_events = visible_person_rows(ordinary_events)
          medication_events = scope.where(record_type: 'Medication', record_id: visible_medications.select(:id))
          pause_events = scope.where(record_type: 'MedicationPausePeriod', record_id: visible_pause_period_ids)
          ordinary_events.or(medication_events).or(pause_events).order(:occurred_at, :id)
        end

        def tombstones_since(since)
          scope = ApiTombstone.where(household: current_household).where.not(record_type: 'MedicationPausePeriod')
                              .where(deleted_at: since..)
          ordinary_tombstones = visible_person_rows(scope.where.not(record_type: 'Medication'))
          medication_tombstones = visible_medication_tombstones(scope)
          ordinary_tombstones.or(medication_tombstones).order(:deleted_at, :id)
        end

        def visible_person_rows(scope)
          visible_people = policy_scope(Person).select(:portable_id)
          household_rows = scope.where(record_type: HOUSEHOLD_WIDE_RECORD_TYPES)
          related_person_rows = scope.where(record_type: PERSON_SCOPED_RECORD_TYPES - ['Person'])
                                     .where("metadata ->> 'person_portable_id' IN (?)", visible_people)
          person_rows = scope.where(record_type: 'Person', record_portable_id: visible_people)
          legacy_person_rows = household_manager_authorized? ? scope.where(record_type: PERSON_SCOPED_RECORD_TYPES) : scope.none

          household_rows.or(related_person_rows).or(person_rows).or(legacy_person_rows)
        end

        def visible_medication_tombstones(scope)
          medication_rows = scope.where(record_type: 'Medication')
          return medication_rows if household_manager_authorized?

          visible_person_ids = policy_scope(Person).pluck(:portable_id)
          person_rows = medication_rows.where(
            "jsonb_exists_any(metadata -> 'sync_person_portable_ids', ARRAY(SELECT jsonb_array_elements_text(?::jsonb)))",
            visible_person_ids.to_json
          )
          creator_rows = medication_rows.where(
            "metadata ->> 'sync_creator_membership_id' = ?", current_membership.id.to_s
          )

          person_rows.or(creator_rows)
        end

        def household_manager_authorized?
          current_membership.owner? || current_membership.administrator?
        end

        def visible_medications
          medications = Medication.where(household: current_household)
          authorized_ids = policy_scope(Medication).select(:id)
          health_event_ids = HealthEventMedication.where(health_event_id: policy_scope(HealthEvent).select(:id))
                                                  .where.not(medication_id: nil)
                                                  .select(:medication_id)

          medications.where(id: authorized_ids).or(medications.where(id: health_event_ids))
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
            metadata: public_metadata(tombstone.metadata)
          }
        end

        def public_metadata(metadata)
          metadata.except(
            'sync_person_portable_ids', 'sync_creator_membership_id',
            :sync_person_portable_ids, :sync_creator_membership_id
          )
        end
      end
    end
  end
end
