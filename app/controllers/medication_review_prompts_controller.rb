# frozen_string_literal: true

class MedicationReviewPromptsController < ApplicationController
  REVIEW_STATUS_FILTERS = %w[needs_review reviewed all].freeze
  PRIORITY_FILTERS = %w[all discuss_soon ask_when_convenient low_confidence].freeze

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
    prompts = filter_by_priority(filter_by_review_status(visible_scope)).order(:person_id, :created_at, :id).to_a
    replace_prompt_with_errors(prompts, prompt_with_errors)

    render Components::MedicationReviews::IndexView.new(
      prompts: prompts,
      filters: {
        hidden_count: scope.hidden_low_signal.count,
        show_hidden: show_hidden?,
        review_status: review_status_filter,
        priority: priority_filter,
        review_counts: review_counts(visible_scope)
      }
    ), status: status
  end

  def filter_by_review_status(scope)
    case review_status_filter
    when 'needs_review' then scope.where(status: unresolved_statuses)
    when 'reviewed' then scope.where(status: reviewed_statuses)
    else scope
    end
  end

  def filter_by_priority(scope)
    case priority_filter
    when 'discuss_soon' then scope.where(risk_level: 'high')
    when 'ask_when_convenient' then scope.where(risk_level: 'moderate')
    when 'low_confidence'
      scope.where(risk_level: %w[low unknown]).or(scope.where(match_confidence: %w[low unknown]))
    else scope
    end
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
