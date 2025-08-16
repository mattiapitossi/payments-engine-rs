fmt:
  cargo fmt

check:
  cargo fmt --all -- --check && cargo clippy && cargo test && cargo audit

