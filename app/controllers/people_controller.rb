# frozen_string_literal: true

class PeopleController < ApplicationController
  include PersonViewable

  before_action :set_person, only: %i[show update destroy add_medication]

  def index
    authorize Person
    people = PeopleIndexQuery.new(scope: policy_scope(Person)).call
    render Components::People::IndexView.new(people: people)
  end

  def show
    authorize @person
    show_data = PersonShowQuery.new(person: @person).call

    render Components::People::ShowView.new(
      person: @person,
      schedules: show_data.schedules,
      person_medications: show_data.person_medications,
      current_user: current_user
    )
  end

  def new
    @person = Person.new
    authorize @person
    assigned_location = primary_location

    render_modal_or_page(
      modal: -> { Components::People::Modal.new(person: @person, assigned_location: assigned_location) },
      page: -> { Components::People::FormView.new(person: @person, assigned_location: assigned_location) }
    )
  end

  def edit
    @person = policy_scope(Person).find(params.expect(:id))
    authorize @person
    @return_to = url_from(params[:return_to])

    render_modal_or_page(
      modal: -> { Components::People::Modal.new(person: @person, return_to: @return_to) },
      page: -> { Components::People::FormView.new(person: @person, return_to: @return_to) }
    )
  end

  def create
    @person = Person.new(person_params)
    @person.household = current_household if current_household
    authorize @person
    @person.primary_location = primary_location

    respond_to do |format|
      if persist_created_person
        format.html { redirect_to @person, notice: t('people.created') }
        format.turbo_stream do
          flash.now[:notice] = t('people.created')
          render turbo_stream: [
            turbo_stream.update('modal', ''),
            turbo_stream.prepend(tenant_dom_target('people'), Components::People::PersonCard.new(person: @person.reload)),
            turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
          ]
        end
      else
        format.html do
          render Components::People::FormView.new(person: @person, assigned_location: primary_location),
                 status: :unprocessable_content
        end
        format.turbo_stream do
          render turbo_stream: turbo_stream.replace(
            'modal',
            Components::People::Modal.new(person: @person, assigned_location: primary_location)
          ), status: :unprocessable_content
        end
      end
    end
  end

  def update
    authorize @person
    respond_to do |format|
      if @person.update(person_params)
        format.html { redirect_to safe_redirect_path(params[:return_to]) || @person, notice: t('people.updated') }
        format.turbo_stream do
          flash.now[:notice] = t('people.updated')
          render turbo_stream: [
            turbo_stream.update('modal', ''),
            turbo_stream.replace(tenant_dom_id(@person), Components::People::PersonCard.new(person: @person.reload)),
            turbo_stream.replace(tenant_dom_target("person_show_#{@person.id}"), person_show_view(@person.reload)),
            turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
          ]
        end
        format.json { render :show, status: :ok, location: @person }
      else
        format.html do
          render Components::People::FormView.new(person: @person, return_to: url_from(params[:return_to])), status: :unprocessable_content
        end
        format.turbo_stream do
          render turbo_stream: turbo_stream.replace(
            'modal',
            Components::People::Modal.new(person: @person, return_to: url_from(params[:return_to]))
          ), status: :unprocessable_content
        end
        format.json { render json: @person.errors, status: :unprocessable_content }
      end
    end
  end

  # DELETE /people/1 or /people/1.json
  def destroy
    authorize @person
    destroy_person
  end

  def add_medication
    authorize @person, :add_medication?

    render Components::People::AddMedicationLanding.new(
      person: @person,
      can_schedule: policy(add_medication_schedule).new?,
      can_person_medication: policy(add_medication_person_medication).new?,
      back_path: add_medication_back_path,
      medication_id: params[:medication_id]
    )
  end

  private

  def persist_created_person
    ActiveRecord::Base.transaction do
      if auto_assign_created_person_carer_relationship?
        CareDelegation::Assign.new(
          carer: current_membership.person,
          patient: @person,
          relationship_type: :family_member,
          granted_by_membership: current_membership
        ).call
      else
        @person.save!
        grant_created_person_access
      end
    end
    true
  rescue ActiveRecord::RecordInvalid => e
    @person.errors.merge!(e.record.errors) unless e.record == @person
    false
  rescue CareDelegation::Assign::Error => e
    @person.errors.add(:base, e.message)
    false
  end

  def destroy_person
    if MedicationAdministrationHistory.exists_for?(@person)
      @person.errors.add(:base, 'Person cannot be deleted while administration history exists')
      return render_destroy_failure
    end

    return render_destroy_failure unless @person.destroy

    respond_to do |format|
      format.html { redirect_to people_path, status: :see_other, notice: t('people.deleted') }
      format.json { head :no_content }
    end
  end

  def set_person
    @person = policy_scope(Person).find(params.expect(:id))
  end

  def person_params
    params.expect(person: %i[name date_of_birth email person_type has_capacity])
  end

  def primary_location
    PrimaryLocationQuery.new(person: current_user&.person).call
  end

  def add_medication_schedule
    @add_medication_schedule ||= @person.schedules.build(medication_id: params[:medication_id])
  end

  def add_medication_person_medication
    @add_medication_person_medication ||= @person.person_medications.build(medication_id: params[:medication_id])
  end

  def add_medication_back_path
    return add_medication_path(medication_id: params[:medication_id]) if params[:source] == 'workflow'

    person_path(@person)
  end

  def grant_created_person_access
    return unless current_household && current_membership

    access_change.create_grant!(
      household: current_household,
      household_membership: current_membership,
      person: @person,
      access_level: :manage,
      relationship_type: :family_member,
      granted_by_membership: current_membership
    )
  end

  def access_change
    @access_change ||= Households::AccessChange.new(
      actor_account: current_account,
      actor_membership: current_membership,
      request: request
    )
  end

  def auto_assign_created_person_carer_relationship?
    current_membership&.person.present? && (@person.minor? || @person.dependent_adult?)
  end

  def render_destroy_failure
    message = @person.errors.full_messages.to_sentence.presence || 'Person could not be deleted'
    respond_to do |format|
      format.html { redirect_to @person, alert: message, status: :see_other }
      format.turbo_stream do
        flash.now[:alert] = message
        render turbo_stream: turbo_stream.update(
          'flash',
          Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert])
        ), status: :unprocessable_content
      end
      format.json { render json: @person.errors, status: :unprocessable_content }
    end
  end
end
