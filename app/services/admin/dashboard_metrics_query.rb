# frozen_string_literal: true

module Admin
  class DashboardMetricsQuery
    def call
      {
        total_users: User.count,
        active_users: User.active.count,
        recent_signups: User.where(created_at: 7.days.ago..).count,
        users_by_role: User.group(:role).count,
        total_people: Person.count,
        people_by_type: Person.group(:person_type).count,
        active_schedules: Schedule.where(active: true).count,
        patients_without_carers: patients_without_carers_count
      }
    end

    private

    def patients_without_carers_count
      without_capacity = Person.where(has_capacity: false)

      without_capacity.where.missing(:carer_relationships)
                      .or(
                        without_capacity.left_joins(:carer_relationships)
                                        .where(carer_relationships: { active: false })
                      )
                      .distinct
                      .count
    end
  end
end
