# frozen_string_literal: true

module ScheduleResourceResolvable
  extend ActiveSupport::Concern

  private

  def set_schedule
    @schedule = policy_scope(Schedule).find(params[:id])
  end

  def schedule_params
    params.expect(schedule: %i[medication_id dosage_id frequency start_date end_date notes max_daily_doses min_hours_between_doses dose_cycle custom_dose_amount custom_dose_unit])
  end
end
