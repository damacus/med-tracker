# Long-content verification report

This bounded step used the isolated Docker test overlay and Playwright browser. Canary was not
used. The affected test file was `spec/system/mobile_overflow_spec.rb`.

## Passing candidate

The mobile rail was tested with the three eligible admin shortcuts `finder`, `medicine_reviews`,
and `administration` at 320px and 390px for English (`en`), Portuguese (`pt`), and Welsh (`cy`).
Each run checked label rectangles, label scroll containment, link bounds, a minimum 44px control
height, and page overflow. All shortcut cases passed; no production change is proposed for this
candidate.

## Red candidate

The invitation check creates a syntactically valid pending invitation whose local part is 64
characters (`long` plus 60 `x` characters) and checks the rendered invitation row at 320px and 390px.
The initial run was:

```text
env COMPOSE_FILE=compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml task test TEST_FILE=spec/system/mobile_overflow_spec.rb
```

The focused aggregate reproduction was:

```text
env COMPOSE_FILE=compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml task test TEST_FILE=spec/system/mobile_overflow_spec.rb:243
```

It produced **1 example, 1 failure**, with both widths failing the same text-containment predicate:

```text
320px: row 16..304 (288px), email paragraph 48..272 (224px), emailContained=false,
       emailWithinRow=true, actionsReachable=true, pageOverflow=0
390px: row 16..374 (358px), email paragraph 48..342 (294px), emailContained=false,
       emailWithinRow=true, actionsReachable=true, pageOverflow=0
```

The page does not horizontally scroll and the resend/cancel actions remain reachable, but the
unbroken email token exceeds the paragraph's own content box. The smallest proposed correction is
to add a wrapping utility such as `break-words` to the email paragraph in
`app/components/admin/invitations/index_view.rb`. No production correction has been applied pending
scope assignment.

The browser failure artifact is
`tmp/capybara/failures_r_spec_example_groups_mobile_overflow_handling_contains_a_long_invitation_email_while_keeping_row_actions_reachable_77.png`.

## Correction and green evidence

The approved narrow correction adds `break-all` to the invitation email paragraph in
`app/components/admin/invitations/index_view.rb`. This preserves the complete email while allowing
an unbroken local part to wrap inside its existing row.

The focused invitation check then passed:

```text
env COMPOSE_FILE=compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml task test TEST_FILE=spec/system/mobile_overflow_spec.rb:243
1 example, 0 failures
```

The complete affected overflow file, including all existing checks and the English/Portuguese/Welsh
shortcut matrix, passed:

```text
env COMPOSE_FILE=compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml task test TEST_FILE=spec/system/mobile_overflow_spec.rb
9 examples, 0 failures
```

No other production files were changed for this candidate step.
