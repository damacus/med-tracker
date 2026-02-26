# frozen_string_literal: true

module Admin
  # Handles admin dashboard functionality
  class DashboardController < ApplicationController
    def index
      authorize :admin_dashboard, :index?

      metrics = {
        total_users: User.count,
        active_users: User.active.count,
        recent_signups: User.where(created_at: 7.days.ago..).count,
        users_by_role: User.group(:role).count,
        total_people: Person.count,
        people_by_type: Person.group(:person_type).count,
        active_schedules: Schedule.where(active: true).count,
        patients_without_carers: patients_without_carers_count
      }

      render Components::Admin::Dashboard::IndexView.new(metrics: metrics)
    end

    private

    def patients_without_carers_count
      # Find people without capacity who don't have active carer relationships
      Person.where(has_capacity: false)
            .where.missing(:carer_relationships)
            .or(
              Person.where(has_capacity: false)
                    .left_joins(:carer_relationships)
                    .where(carer_relationships: { active: false })
            )
            .distinct
            .count
    end
  end
end
