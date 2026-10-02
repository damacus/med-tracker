# frozen_string_literal: true

class MedicationReviewPromptsController < ApplicationController
  REVIEW_STATUS_FILTERS = MedicationReviewPromptQuery::REVIEW_STATUS_FILTERS
  PRIORITY_FILTERS = MedicationReviewPromptQuery::PRIORITY_FILTERS

  def index
    authorize MedicationReviewPrompt
    render_index
  end

  def update
    prompt = policy_scope(MedicationReviewPrompt).find(params.expect(:id))
    authorize prompt
    prompt.assign_attributes(prompt_params)
    prompt.reviewed_by_membership = current_membership if prompt.practitioner_review_status?

    if prompt.save
      redirect_to medication_review_prompts_path, notice: t('medication_reviews.updated')
    else
      render_index(prompt_with_errors: prompt, status: :unprocessable_content)
    end
  end

  private

  def render_index(prompt_with_errors: nil, status: :ok)
    people = policy_scope(Person)
    MedicationReviewPromptSync.new(people: people).call
    scope = policy_scope(MedicationReviewPrompt).includes(:person, :primary_medication, :interacting_medication)
    visible_scope = show_hidden? ? scope : scope.visible_by_default
    prompts = MedicationReviewPromptQuery.new(scope: scope, review_status: review_status_filter,
                                              priority: priority_filter, show_hidden: show_hidden?).call.to_a
    replace_prompt_with_errors(prompts, prompt_with_errors)

    render Components::MedicationReviews::IndexView.new(
      prompts: prompts,
      filters: {
        hidden_count: scope.except(:includes).hidden_low_signal.count,
        show_hidden: show_hidden?,
        review_status: review_status_filter,
        priority: priority_filter,
        review_counts: review_counts(visible_scope)
      }
    ), status: status
  end

  def review_counts(scope)
    {
      needs_review: scope.where(status: unresolved_statuses).count,
      reviewed: scope.where(status: reviewed_statuses).count,
      all: scope.count
    }
  end

  def unresolved_statuses
    show_hidden? ? %w[needs_review hidden_low_signal] : %w[needs_review]
  end

  def reviewed_statuses
    MedicationReviewPrompt::STATUSES - %w[needs_review hidden_low_signal]
  end

  def review_status_filter
    params.fetch(:review_status, nil).presence_in(REVIEW_STATUS_FILTERS) || 'needs_review'
  end

  def priority_filter
    params.fetch(:priority, nil).presence_in(PRIORITY_FILTERS) || 'all'
  end

  def replace_prompt_with_errors(prompts, prompt_with_errors)
    return unless prompt_with_errors

    index = prompts.index { |prompt| prompt.id == prompt_with_errors.id }
    prompts[index] = prompt_with_errors if index
  end

  def show_hidden?
    params[:show_hidden] == '1'
  end

  def prompt_params
    params.expect(
      medication_review_prompt: %i[status practitioner_name practitioner_role reviewed_on review_note]
    )
  end
end
