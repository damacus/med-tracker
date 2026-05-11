# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Profiles::VersionInfo, type: :component do
  it 'renders worktree and commit metadata instead of the app version' do
    allow(SystemMetadata).to receive(:current).and_return(
      SystemMetadata.new(
        worktree: '/Users/damacus/.codex/worktrees/a270/med-tracker',
        commit: 'bb956afc'
      )
    )

    rendered = render_inline(described_class.new)
    html = rendered.to_html

    expect(html).to include('Worktree')
    expect(html).to include('/Users/damacus/.codex/worktrees/a270/med-tracker')
    expect(html).to include('Commit')
    expect(html).to include('bb956afc')
    expect(html).not_to include('v0.3.51')
  end

  it 'uses token-driven shells for version info' do
    rendered = render_inline(described_class.new)
    html = rendered.to_html

    banned_classes = ['rounded-[2rem]', 'bg-card/95', 'shadow-[0_18px_45px_-32px_rgba']

    expect(banned_classes.none? { |class_name| html.include?(class_name) }).to be(true)
  end
end
