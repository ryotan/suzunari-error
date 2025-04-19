# Development Guide

This document provides guidelines for contributors to this project. It includes important information you should know when participating in the project, such as how to write code, commit rules, how to create PRs, and more.

## Directory Structure

```
<root>/
├── app/                      # フロントエンドのソースコード
│   │                         # Frontend source code
│   ├── src/                  # Reactコンポーネント、スタイル、ルーティングなど
│   │                         # React components, styles, routing, etc.
│   ├── public/               # 静的アセット
│   │                         # Static assets
│   ├── index.html            # HTMLエントリポイント
│   │                         # HTML entry point
│   ├── package.json          # npm依存関係とスクリプト
│   │                         # npm dependencies and scripts
│   └── vite.config.ts        # Viteの設定
│                             # Vite configuration
├── app/src-tauri/            # Tauriバックエンドのソースコード
│   │                         # Tauri backend source code
│   ├── src/                  # Rustのソースコード
│   │                         # Rust source code
│   ├── Cargo.toml            # Rustの依存関係
│   │                         # Rust dependencies
│   └── tauri.conf.json       # Tauriの設定
│                             # Tauri configuration
├── CONTRIBUTING.md           # 貢献ガイドライン
│                             # Contribution guidelines
└── README.md                 # プロジェクトの概要とセットアップ手順
                              # Project overview and setup instructions
```

## Development Environment Setup

1. Clone the repository:
   ```
   git clone https://github.com/ryotan/suzunari-error.git
   cd suzunari-error
   ```


## Coding Conventions

See [Rust Style Guide](./.junie/guidelines-rs.md)

## Commit Message Conventions

See [Commit Message Conventions](./.junie/guidelines-git.md)

## Creating Pull Requests (PRs)

1. Create a new branch (include feature name or bug fix name):
   ```
   git checkout -b feat/git-diff-viewer
   ```

2. Implement changes and commit:
   ```
   git add .
   git commit -m "feat: Gitの差分表示機能を追加"
   ```

3. Push to remote branch:
   ```
   git push -u origin feat/git-diff-viewer
   ```

4. Create a pull request on GitHub. Include the following in the PR description:

   - Summary of changes
   - Related issue number
   - How to test
   - Screenshots (if there are UI changes)

## Testing

See "Testing" section in [Rust Style Guide](./.junie/guidelines-rs.md)

- Run Rust tests with `mise test`.
- Use `mise clippy` for code quality checks.
- To ensure code quality, be sure to run `mise lint` and `mise test`. These commands run code quality checks and tests to ensure there are no issues.

## Release

1. The `main` branch is always kept in a stable state.
2. Releases are made by tagging: `v1.0.0`, `v1.0.1`, etc.
3. Follow semantic versioning:
   - Major: Incompatible changes
   - Minor: Backward-compatible feature additions
   - Patch: Backward-compatible bug fixes

## Help

If you have questions or need assistance, create an Issue or contact the project administrator.

---

Thank you for your cooperation!
