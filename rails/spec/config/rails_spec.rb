require 'rails_helper'

RSpec.describe Rails do
  let(:repository_root) { Pathname.new(ENV.fetch('MEDTRACKER_REPOSITORY_ROOT', described_class.root.parent.to_s)) }

  it 'resolves shared CI through the Rails symlink to the canonical source' do
    rails_path = described_class.root.join('scripts/ci/classify.mjs')
    shared_path = repository_root.join('scripts/ci/classify.mjs')

    expect(rails_path).to exist
    expect(shared_path).to exist
    expect(File.identical?(rails_path, shared_path)).to be(true)
  end

  it 'exposes repository-owned dependency policy independently of Rails' do
    expect(repository_root.join('renovate.json')).to exist
  end

  it 'preserves canonical generated-source exclusions for Tailwind scanning' do
    rails_ignore = described_class.root.join('.gitignore')
    shared_ignore = repository_root.join('.gitignore')

    expect(rails_ignore).to exist
    expect(shared_ignore).to exist
    expect(File.identical?(rails_ignore, shared_ignore)).to be(true)
    expect(rails_ignore.read.lines.map(&:chomp)).to include('/tmp/*', '/rust/api/target/')
  end
end
