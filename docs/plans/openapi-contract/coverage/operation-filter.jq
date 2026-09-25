def refname:
  if type == "object" and has("$ref") then .["$ref"] | split("/")[-1] else null end;
def deref($spec):
  if type == "object" and has("$ref") then
    .["$ref"] as $ref
    | $spec | getpath($ref | ltrimstr("#/") | split("/") | map(gsub("~1";"/") | gsub("~0";"~")))
  else . end;
. as $spec
| .paths | to_entries[]
| .key as $path
| .value | to_entries[]
| select(.key | test("^(get|post|put|patch|delete|options|head|trace)$"))
| .key as $method
| .value as $op
| ($routes[0][$op.operationId] // null) as $source
| ($evidence[0][$op.operationId] // null) as $test
| {
    operation_id: ($op.operationId // null), method: $method, path: $path,
    server_base: $spec.servers[0].url,
    spec_pointer: ("#/paths/" + ($path | gsub("~";"~0") | gsub("/";"~1")) + "/" + $method),
    request_schema: (($op.requestBody.content."application/json".schema // null) | refname),
    request_schema_required: (($op.requestBody.content."application/json".schema // null) | deref($spec) | .required // []),
    request_schema_properties: (($op.requestBody.content."application/json".schema // null) | deref($spec) | .properties // {} | keys),
    request_required: ($op.requestBody.required // false),
    responses: ($op.responses | to_entries | map(
      . as $entry | ($entry.value | deref($spec)) as $response | {
        status: $entry.key,
        kind: (if ($entry.key | test("^[45]")) then "error" else "success" end),
        schema: ($response.content."application/json".schema | refname),
        schema_required: ($response.content."application/json".schema | deref($spec) | .required // []),
        schema_properties: ($response.content."application/json".schema | deref($spec) | .properties // {} | keys),
        response_ref: ($entry.value["$ref"] // null),
        headers: (($response.headers // {}) | keys)
      })),
    security: ($op.security // $spec.security),
    parameters: (($op.parameters // []) | map(. | deref($spec) | {name,in,required,schema})),
    implementation: (if $test then {state:"route_present_partial_behaviour_verified",source:$source} elif $source then {state:"route_present_behaviour_unverified",source:$source} else {state:"route_absent",source:null} end),
    contract_tests: ($test // {state:"unverified",source:null}),
    gap: (if $test then {classification:"unassessed",detail:"Only the listed response statuses and behaviour assertions are verified."} elif $source then {classification:"unassessed",detail:"Route presence does not verify schema, security, status, or error behaviour."} else {classification:"implementation",detail:"No Rust API route registered for this documented method and path."} end)
  }
