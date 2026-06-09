# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::Dashboard::IndexView, type: :component do
  fixtures :accounts, :people, :users

  let(:active_schedules_icon_path) do
    [
      'M200-640h560v-80H200v80Zm0 0v-80 80Zm0 560q-33 0-56.5-23.5T120-160v-560q0-33 ',
      '23.5-56.5T200-800h40v-80h80v80h320v-80h80v80h40q33 0 56.5 23.5T840-720v227q-19-9-39-15t-41-9v-43H200v400h252q7 ',
      '22 16.5 42T491-80H200Zm378.5-18.5Q520-157 520-240t58.5-141.5Q637-440 ',
      '720-440t141.5 58.5Q920-323 920-240T861.5-98.5Q803-40 ',
      '720-40T578.5-98.5ZM787-145l28-28-75-75v-112h-40v128l87 87Z'
    ].join
  end

  it 'renders the active schedules metric with the active schedules icon' do
    rendered = render_inline(
      described_class.new(metrics: { active_schedules: 7 })
    )
    metric = rendered.at_css('[data-testid="metric-active-schedules"]')

    expect(metric.at_css("path[d='#{active_schedules_icon_path}']")).to be_present
  end

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
        href: '/admin/people',
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
        href: '/admin/people',
        action_label: 'View',
        icon_type: 'activity'
      }
    ]

    rendered = render_inline(described_class.new(metrics: { attention_items: items }))

    expect(rendered.text).to include('Patients without carers')
    expect(rendered.at_css("a[href='/admin/people']")).to be_present
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
