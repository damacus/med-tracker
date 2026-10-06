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
        type: 'command',
        title: command.fetch(:title),
        subtitle: command.fetch(:subtitle),
        path: command.fetch(:path),
        score: 50
      )
    end
  end

  private

  def default_url_options
    household = policy_household
    return {} unless household

    { household_slug: household.slug }
  end

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
    Pundit.policy!(policy_user, command.fetch(:record)).public_send(command.fetch(:action))
  rescue Pundit::NotDefinedError
    false
  end

  def policy_user
    AuthorizationContext.current || derived_authorization_context || user
  end

  def derived_authorization_context
    membership = user_account&.first_active_household_membership
    return unless membership

    AuthorizationContext.new(account: user_account, household: membership.household, membership: membership)
  end

  def policy_household
    household_candidates.compact.first
  end

  def household_candidates
    [
      Current.household,
      AuthorizationContext.current&.household,
      derived_authorization_context&.household,
      user.person&.household,
      user_account&.first_active_household
    ]
  end

  def user_account
    user.person&.account
  end
end
