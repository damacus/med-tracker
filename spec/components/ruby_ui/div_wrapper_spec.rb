# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::DivWrapper, type: :component do
  wrappers = {
    RubyUI::AlertDialogFooter => 'flex flex-col-reverse gap-2 sm:flex-row sm:justify-end sm:gap-2',
    RubyUI::AlertDialogHeader => 'flex flex-col gap-2 text-center sm:text-left rtl:sm:text-right',
    RubyUI::CardContent => 'p-6 pt-0',
    RubyUI::CardFooter => 'items-center p-6 pt-0',
    RubyUI::CardHeader => 'flex flex-col space-y-1.5 p-6',
    RubyUI::DialogFooter => 'flex flex-col-reverse sm:flex-row sm:justify-end ' \
                            'sm:space-x-2 gap-y-2 sm:gap-y-0 rtl:space-x-reverse',
    RubyUI::DialogHeader => 'flex flex-col gap-2 border-b border-border/60 bg-popover ' \
                            'px-8 pb-4 pt-8 text-center sm:text-left rtl:sm:text-right',
    RubyUI::DialogMiddle => 'bg-popover px-8 pb-8 pt-4',
    RubyUI::SheetFooter => 'flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2 gap-y-2 sm:gap-y-0',
    RubyUI::SheetHeader => 'flex flex-col space-y-1.5 text-center sm:text-left',
    RubyUI::SheetMiddle => 'py-4'
  }

  wrappers.each do |wrapper, default_classes|
    it "#{wrapper.name} renders a div with merged default and custom classes" do
      rendered = render_inline(wrapper.new(class: 'custom-wrapper') { 'Wrapper body' })
      root = Nokogiri::HTML.fragment(rendered.to_html).element_children.first

      expect(root.name).to eq('div')
      expect(root.text).to include('Wrapper body')
      expect(root['class'].split).to include('custom-wrapper')
      expect(root['class'].split).to include(*default_classes.split)
    end
  end

  it 'falls back to a blank class when a subclass does not set DEFAULT_CLASS' do
    subclass = Class.new(described_class)

    rendered = render_inline(subclass.new(class: 'only-custom') { 'Body' })
    root = Nokogiri::HTML.fragment(rendered.to_html).element_children.first

    expect(root.name).to eq('div')
    expect(root.text).to include('Body')
    expect(root['class'].to_s.split).to include('only-custom')
  end
end
