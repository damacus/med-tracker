class MedicationReviewPromptQuery
  REVIEW_STATUS_FILTERS = %w[needs_review reviewed all].freeze
  PRIORITY_FILTERS = %w[all discuss_soon ask_when_convenient low_confidence].freeze

  def initialize(scope:, review_status: 'needs_review', priority: 'all', show_hidden: false)
    @scope = scope
    @review_status = review_status
    @priority = priority
    @show_hidden = show_hidden
  end

  def call
    raise ArgumentError unless REVIEW_STATUS_FILTERS.include?(@review_status) && PRIORITY_FILTERS.include?(@priority)

    visible = @show_hidden ? @scope : @scope.visible_by_default
    filter_priority(filter_status(visible)).order(:person_id, :created_at, :id)
  end

  private

  def filter_status(scope)
    case @review_status
    when 'needs_review' then scope.where(status: @show_hidden ? %w[needs_review hidden_low_signal] : %w[needs_review])
    when 'reviewed' then scope.where.not(status: %w[needs_review hidden_low_signal])
    else scope
    end
  end

  def filter_priority(scope)
    case @priority
    when 'discuss_soon' then scope.where(risk_level: 'high')
    when 'ask_when_convenient' then scope.where(risk_level: 'moderate')
    when 'low_confidence'
      scope.where(risk_level: %w[low unknown]).or(scope.where(match_confidence: %w[low unknown]))
    else scope
    end
  end
end
