class PreserveImportedPauseActorContext < ActiveRecord::Migration[8.1]
  def change
    add_column :medication_pause_periods, :imported_context, :boolean, null: false, default: false
    add_column :medication_pause_periods, :imported_actor_references, :jsonb, null: false, default: {}
    remove_check_constraint :medication_pause_periods,
                            "(legacy_context AND reason = 'reason_not_recorded') OR " \
                            "(NOT legacy_context AND reason <> 'reason_not_recorded' AND started_at IS NOT NULL " \
                            'AND recorded_by_membership_id IS NOT NULL)',
                            name: 'chk_medication_pause_periods_legacy_context'
    remove_check_constraint :medication_pause_periods,
                            '(ended_at IS NULL AND resumed_by_membership_id IS NULL) OR ' \
                            '(ended_at IS NOT NULL AND resumed_by_membership_id IS NOT NULL)',
                            name: 'chk_medication_pause_periods_resuming_actor'
    add_check_constraint :medication_pause_periods,
                         "(legacy_context AND reason = 'reason_not_recorded') OR " \
                         "(NOT legacy_context AND reason <> 'reason_not_recorded' AND started_at IS NOT NULL " \
                         'AND (recorded_by_membership_id IS NOT NULL OR imported_context))',
                         name: 'chk_medication_pause_periods_legacy_context'
    add_check_constraint :medication_pause_periods,
                         '(ended_at IS NULL AND resumed_by_membership_id IS NULL) OR ' \
                         '(ended_at IS NOT NULL AND (resumed_by_membership_id IS NOT NULL OR imported_context))',
                         name: 'chk_medication_pause_periods_resuming_actor'
  end
end
