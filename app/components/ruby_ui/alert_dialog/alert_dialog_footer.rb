# frozen_string_literal: true

module RubyUI
  class AlertDialogFooter < DivWrapper
    DEFAULT_CLASS = 'flex flex-col-reverse gap-2 sm:flex-row sm:justify-end sm:gap-2 ' \
                    '[&>button]:w-full [&>form]:w-full [&>form>button]:w-full ' \
                    'sm:[&>button]:w-auto sm:[&>form]:w-auto sm:[&>form>button]:w-auto'
  end
end
