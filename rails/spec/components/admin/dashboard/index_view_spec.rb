# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::Dashboard::IndexView, type: :component do
  fixtures :accounts, :people, :users

  it 'renders the all-clear status badge when no attention items exist' do
    rendered = render_inline(described_class.new(metrics: { attention_items: [] }))

    expect(rendered.at_css('[data-testid="dashboard-status"]').text)
      .to include(I18n.t('admin.dashboard.status.all_clear'))
  end

  it 'renders the needs-attention badge with the item count' do
    items = [
      {
        severity: :high,
        title: 'Patients without carers',
        detail: '2 awaiting assignment',
        href: Rails.application.routes.url_helpers.admin_people_path(household_slug: 'test-household'),
        action_label: 'View',
        icon_type: 'activity'
      }
    ]

    rendered = render_inline(described_class.new(metrics: { attention_items: items }))

    expect(rendered.at_css('[data-testid="dashboard-status"]').text)
      .to include(I18n.t('admin.dashboard.status.needs_attention', count: 1))
  end

  it 'renders an attention row with its action link' do
    items = [
      {
        severity: :high,
        title: 'Patients without carers',
        detail: '2 awaiting assignment',
        href: Rails.application.routes.url_helpers.admin_people_path(household_slug: 'test-household'),
        action_label: 'View',
        icon_type: 'activity'
      }
    ]

    rendered = render_inline(described_class.new(metrics: { attention_items: items }))

    expect(rendered.text).to include('Patients without carers')
    expect(rendered.at_css("a[href='/households/test-household/admin/people']")).to be_present
  end

  it 'labels the review section without using queue language' do
    rendered = render_inline(described_class.new(metrics: { attention_items: [] }))

    expect(rendered.at_css('[data-testid="attention-queue"]').text)
      .to include(I18n.t('admin.dashboard.attention.title'))
    expect(rendered.text).not_to include('Attention Queue')
  end

  it 'renders the positive empty row when the queue is clear' do
    rendered = render_inline(described_class.new(metrics: { attention_items: [] }))

    expect(rendered.text).to include(I18n.t('admin.dashboard.attention.empty_title'))
  end

  it 'renders the grouped quick-action headings' do
    rendered = render_inline(described_class.new(metrics: {}))

    expect(rendered.text).to include(I18n.t('admin.dashboard.sections.user_access'))
    expect(rendered.text).to include(I18n.t('admin.dashboard.sections.operations'))
  end

  it 'renders the recent activity section with a humanised row' do
    PaperTrail.request.whodunnit = users(:admin).id
    PaperTrail.request(enabled: true) { people(:john).update!(name: 'Activity Row') }
    version = PaperTrail::Version.order(created_at: :desc).first

    rendered = render_inline(described_class.new(metrics: { recent_activity: [version].compact }))

    expect(rendered.at_css('[data-testid="dashboard-activity"]')).to be_present
    expect(rendered.text).to include(users(:admin).name)
  end
end
