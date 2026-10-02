class MedicationReviewReportQuery
  def initialize(scope:, status: nil)
    @scope = scope
    @status = status
  end

  def call
    raise ArgumentError if @status.present? && MedicationReviewPrompt::STATUSES.exclude?(@status)

    filtered = @status.present? ? @scope.where(status: @status) : @scope.visible_by_default
    filtered.includes(:person).order(:person_id, :created_at, :id)
  end
end
