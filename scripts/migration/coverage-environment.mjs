const overrideNames = [
  'CARGO_LLVM_COV_TARGET_DIR',
  'CARGO_LLVM_COV_BUILD_DIR',
  'CARGO_LLVM_COV_SETUP',
  'CARGO_LLVM_COV_FLAGS',
  'CARGO_LLVM_PROFDATA_FLAGS',
  'LLVM_COV',
  'LLVM_PROFDATA',
  'LLVM_COV_FLAGS',
  'LLVM_PROFDATA_FLAGS',
  'LLVM_PROFILE_FILE',
  'LLVM_PROFILE_FILE_NAME',
];

export function withoutCoverageOverrides(inheritedEnvironment) {
  const environment = { ...inheritedEnvironment };
  for (const name of overrideNames) delete environment[name];
  return environment;
}
