# frozen_string_literal: true

module Api
  module V1
    class DashboardsController < BaseController
      def show
        Time.use_zone(current_household.timezone) do
          date = dashboard_date
          authorize Person, :index?
          visible_people = policy_scope(Person).order(:id).to_a
          selected_people, selected_person = dashboard_people(visible_people)
          server_time = Time.current
          query = FamilyDashboard::ScheduleQuery.new(
            selected_people,
            current_user: pundit_user,
            date: date,
            now: reference_time(date, server_time),
            include_paused: true
          )
          query.call

          render json: {
            data: DashboardSerializer.new(
              DashboardSerializer::Context.new(
                visible_people: visible_people,
                selected_person: selected_person,
                schedule_query: query,
                date: date,
                server_time: server_time,
                time_zone: Time.zone.name
              )
            ).as_json
          }
        end
      end

      private

      def dashboard_date
        value = params[:date]
        return Date.current if value.blank?
        raise InvalidFilterValue, 'date must be YYYY-MM-DD' unless value.to_s.match?(/\A\d{4}-\d{2}-\d{2}\z/)

        Date.iso8601(value)
      rescue Date::Error
        raise InvalidFilterValue, 'date must be YYYY-MM-DD'
      end

      def dashboard_people(visible_people)
        identifier = params[:person_id].presence || 'all'
        return [visible_people, nil] if identifier == 'all'

        person = find_api_record(Person.where(id: visible_people.map(&:id)), identifier)
        [[person], person]
      rescue ActiveRecord::RecordNotFound
        raise InvalidFilterValue, 'person_id is invalid'
      end

      def reference_time(date, server_time)
        return server_time if date == server_time.to_date
        return date.end_of_day if date.past?

        date.beginning_of_day
      end
    end
  end
end
