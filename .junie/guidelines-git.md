# Git Conventions

## Git Branch Conventions

Branch names comply with [Conventional Branch](https://conventional-branch.github.io/).

### Format

The branch specification by describing with `feature/`, `bugfix/`, `hotfix/`, `release/` and `chore/` and it should be structured as follows:

```
<type>/<description>
```

* feature/: For new features (e.g., feature/add-login-page)
* bugfix/: For bug fixes (e.g., bugfix/fix-header-bug)
* hotfix/: For urgent fixes (e.g., hotfix/security-patch)
* release/: For branches preparing a release (e.g., release/v1.2.0)
* chore/: For non-code tasks like dependency, docs updates (e.g., chore/update-dependencies)

### Naming Conventions

* **Use Lowercase Alphanumeric and Hyphens**: Always use lowercase letters (a-z), numbers (0-9), and hyphens to separate
words. Avoid special characters, underscores, or spaces.
* **No Consecutive or Trailing Hyphens**: Ensure that hyphens are used singly, with no consecutive hyphens (
feature/new--login) or at the end (feature/new-login-).
* **Keep It Clear and Concise**: The branch name should be descriptive yet concise, clearly indicating the purpose of the
work.
* **Include Ticket Numbers**: If applicable, include the ticket number from your project management tool to make tracking
easier. For example, for a ticket issue-123, the branch name could be feature/issue-123-new-login.

## Commit Message Conventions

Commit messages comply with Conventional Commits, using [GitMoji](https://gitmoji.dev/) with Unicode Emoji input

### Format

```
`<emoji>: <summary>`

* <description 1 (optional)>
* ...
```

Example:

```
✨: Add Git diff display feature

- Display uncommitted changes
- Display staged changes
- Display untracked files
```

### GitMoji Categories

GitMoji are organized into categories to help you find the appropriate emoji for your commit message.

#### 🚀 Features and Improvements

- ✨: Introduce new features.
- 🚀: Deploy stuff.
- 💄: Add or update the UI and style files.
- 🎉: Begin a project.
- 🚸: Improve user experience / usability.
- 📱: Work on responsive design.
- 🥚: Add or update an easter egg.
- 💫: Add or update animations and transitions.
- 👔: Add or update business logic.
- ✈️: Improve offline support.
- 🚩: Add, update, or remove feature flags.

#### 🐛 Bug Fixes and Critical Changes

- 🐛: Fix a bug.
- 🚑️: Critical hotfix.
- 🩹: Simple fix for a non-critical issue.
- 🔒️: Fix security or privacy issues.
- 🥅: Catch errors.
- 💥: Introduce breaking changes.

#### 🧹 Code Quality and Maintenance

- 🎨: Improve structure / format of the code.
- ⚡️: Improve performance.
- 🔥: Remove code or files.
- ♻️: Refactor code.
- ✏️: Fix typos.
- 💩: Write bad code that needs to be improved.
- 🗑️: Deprecate code that needs to be cleaned up.
- ⚰️: Remove dead code.
- 🚨: Fix compiler / linter warnings.
- 🧑‍💻: Improve developer experience.
- 🏗️: Make architectural changes.

#### 📝 Documentation and Comments

- 📝: Add or update documentation.
- 💡: Add or update comments in source code.
- 💬: Add or update text and literals.
- 📄: Add or update license.
- 👥: Add or update contributor(s).

#### 🧪 Testing and Validation

- ✅: Add, update, or pass tests.
- 🧪: Add a failing test.
- 📸: Add or update snapshots.
- ⚗️: Perform experiments.
- 🦺: Add or update code related to validation.
- 🩺: Add or update healthcheck.
- 🧐: Data exploration/inspection.
- 🤡: Mock things.

#### 📦 Dependencies and Assets

- ⬇️: Downgrade dependencies.
- ⬆️: Upgrade dependencies.
- 📌: Pin dependencies to specific versions.
- ➕: Add a dependency.
- ➖: Remove a dependency.
- 🍱: Add or update assets.
- 📦️: Add or update compiled files or packages.
- 🌱: Add or update seed files.

#### ⚙️ Configuration and Infrastructure

- 🔧: Add or update configuration files.
- 🔨: Add or update development scripts.
- 🙈: Add or update a .gitignore file.
- 🧱: Infrastructure related changes.
- 💸: Add sponsorships or money related infrastructure.
- 🔐: Add or update secrets.

#### 🔄 Version Control and CI/CD

- 🚧: Work in progress.
- ⏪️: Revert changes.
- 🔀: Merge branches.
- 💚: Fix CI Build.
- 👷: Add or update CI build system.
- 🔖: Release / Version tags.

#### 🌐 Internationalization and Data

- 🌐: Internationalization and localization.
- 🗃️: Perform database related changes.
- 🔊: Add or update logs.
- 🔇: Remove logs.
- 📈: Add or update analytics or track code.
- 🔍️: Improve SEO.

#### 🧵 Advanced Programming

- 🧵: Add or update code related to multithreading or concurrency.
- 🏷️: Add or update types.
- 👽️: Update code due to external API changes.
- 🚚: Move or rename resources (e.g.: files, paths, routes).
- 🛂: Work on code related to authorization, roles and permissions.
- ♿️: Improve accessibility.

#### 🍻 Miscellaneous

- 🍻: Write code drunkenly.
