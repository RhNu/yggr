# AGENTS.md

## 文档规范

- 禁止使用 Emoji 符号
- 文档尽量简短
- 通常不写日期
- 不给文档编号

## Rust 开发门禁

提交前必须依次通过以下三项,且无任何警告:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

发现警告必须修复至零警告为止。

## 前端开发门禁

提交前必须依次通过以下三项,且无任何警告:

```bash
pnpm --filter frontend lint
pnpm --filter frontend format:check
pnpm --filter frontend build
```

发现警告必须修复至零警告为止。
