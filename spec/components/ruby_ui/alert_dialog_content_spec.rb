# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::AlertDialogContent, type: :component do
  it 'uses token-driven floating surfaces instead of hard-coded white' do
    rendered = render_inline(described_class.new { 'Alert body' })
    html = rendered.to_html

    expect(html).to include('bg-popover')
    expect(html).to include('bg-foreground/10')
    expect(html).to include('backdrop-blur-[1.5px]')
    expect(html).not_to include('bg-white')
    expect(html).not_to include('bg-black/80')
  end

  it 'renders a named modal alert dialog surface' do
    body = [
      RubyUI::AlertDialogTitle.new { 'Delete medication?' }.render_in(view_context),
      RubyUI::AlertDialogDescription.new { 'This cannot be undone.' }.render_in(view_context)
    ]
    rendered = render_inline(described_class.new { view_context.safe_join(body) })
    dialog = rendered.at_css('dialog[role="alertdialog"]')

    expect(dialog).to be_present
    expect([dialog['aria-modal'], dialog['aria-labelledby'], dialog['aria-describedby']]).to eq(
      ['true', nil, nil]
    )
    labelled_nodes = dialog.css('[data-ruby-ui-alert-dialog-title], [data-ruby-ui-alert-dialog-description]')
    expect(labelled_nodes.map(&:text)).to eq(
      ['Delete medication?', 'This cannot be undone.']
    )
  end

  it 'does not reference a title or description that was not rendered' do
    rendered = render_inline(described_class.new { 'Alert body' })
    dialog = rendered.at_css('[role="alertdialog"]')

    expect(dialog['aria-labelledby']).to be_nil
    expect(dialog['aria-describedby']).to be_nil
  end

  it 'uses native cancellation rather than a template portal' do
    rendered = render_inline(described_class.new { 'Alert body' })

    expect(rendered.at_css('template')).to be_nil
    expect(rendered.at_css('dialog')['data-action']).to include(
      'cancel->ruby-ui--alert-dialog#dismiss',
      'keydown->ruby-ui--alert-dialog#trapFocus'
    )
  end

  it 'does not render duplicate title and description ids across alert dialogs' do
    contents = 2.times.map do
      title = RubyUI::AlertDialogTitle.new { 'Delete medication?' }.render_in(view_context)
      description = RubyUI::AlertDialogDescription.new { 'This cannot be undone.' }.render_in(view_context)

      described_class.new { view_context.safe_join([title, description]) }.render_in(view_context)
    end
    ids = Nokogiri::HTML.fragment(contents.join).css('[id]').pluck('id')

    expect(ids).to eq(ids.uniq)
  end
end
