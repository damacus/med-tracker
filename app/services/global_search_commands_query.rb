# frozen_string_literal: true

class GlobalSearchCommandsQuery
  include Rails.application.routes.url_helpers

  attr_reader :user

  def initialize(user:)
    @user = user
  end

  def call
    command_definitions.filter_map do |command|
      next unless command_allowed?(command)

      GlobalSearchQuery::Result.new(
        type: "command",
        title: command.fetch(:title),
        subtitle: command.fetch(:subtitle),
        path: command.fetch(:path),
        score: 50
      )
    end
  end

  private

  def command_definitions
    navigation_command_definitions + creation_command_definitions
  end

  def navigation_command_definitions
    [
      command(:medication_finder, medication_finder_path, Medication, :finder?),
      command(:inventory, medications_path, Medication, :index?),
      command(:people, people_path, Person, :index?),
      command(:locations, locations_path, Location, :index?),
      command(:reports, reports_path, :report, :index?)
    ]
  end

  def creation_command_definitions
    [
      command(:add_medication, add_medication_path, Medication, :create?),
      command(:add_person, new_person_path, Person.new, :new?),
      command(:schedule_workflow, schedules_workflow_path, Schedule.new(person: user.person), :create?)
    ]
  end

  def command(key, path, record, action)
    {
      title: I18n.t("global_search.commands.#{key}.title"),
      subtitle: I18n.t("global_search.commands.#{key}.subtitle"),
      path: path,
      record: record,
      action: action
    }
  end

  def command_allowed?(command)
    Pundit.policy!(user, command.fetch(:record)).public_send(command.fetch(:action))
  rescue Pundit::NotDefinedError
    false
  end
end
