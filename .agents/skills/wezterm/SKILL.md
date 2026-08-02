```markdown
# wezterm Development Patterns

> Auto-generated skill from repository analysis

## Overview

This skill teaches the primary development patterns, coding conventions, and maintenance workflows used in the `wezterm` codebase—a modern terminal emulator written in Rust. You will learn how to structure code, follow commit and file naming conventions, and execute common repository workflows such as updating CI, documentation, and Nix flakes, as well as handling Wayland-specific refactors and testing.

## Coding Conventions

- **File Naming:**  
  Use `snake_case` for all Rust source files.
  ```
  // Good
  src/font_config.rs
  window/src/os/wayland/copy_and_paste.rs

  // Bad
  src/FontConfig.rs
  window/src/os/wayland/CopyAndPaste.rs
  ```

- **Import Style:**  
  Use relative imports within modules.
  ```rust
  // In src/foo/bar.rs
  use super::baz;
  use crate::utils::helper;
  ```

- **Export Style:**  
  Use named exports for modules and functions.
  ```rust
  pub fn do_something() { ... }
  pub struct MyStruct { ... }
  ```

- **Commit Patterns:**  
  - Mixed types; common prefixes: `docs`, `fix`, `ci`, `nix`, `wayland`
  - Average commit message length: ~49 characters

## Workflows

### Update CI Workflows
**Trigger:** When CI platforms, dependencies, or build logic change and workflows must be regenerated or updated.  
**Command:** `/update-ci-workflows`

1. Edit or run `ci/generate-workflows.py` to update workflow templates.
2. Regenerate or manually edit `.github/workflows/gen_*.yml` files.
3. Commit all changed workflow files together.

### Document Installation Instructions
**Trigger:** When support for a new distro is added or installation steps change.  
**Command:** `/update-install-docs`

1. Edit `docs/install/linux.md` to add or update installation steps.
2. Optionally update related badges or links.

### Update Changelog After Feature or Fix
**Trigger:** After merging a notable feature, fix, or documentation update.  
**Command:** `/update-changelog`

1. Edit `docs/changelog.md` to summarize the changes.
2. Commit with reference to the related PR or feature.

### Nix Flake Update and Fix
**Trigger:** When Nix dependencies change, or Nix workflows need fixes or modernization.  
**Command:** `/nix-flake-update`

1. Update `nix/flake.lock` (e.g., via `nix flake update`).
2. Edit `nix/flake.nix` for code or dependency changes.
3. Optionally update related GitHub Actions workflows.

### Wayland Clipboard or DnD Refactor
**Trigger:** When improving, fixing, or restructuring Wayland clipboard or DnD logic.  
**Command:** `/wayland-clipboard-refactor`

1. Edit relevant files in `window/src/os/wayland/` (e.g., `copy_and_paste.rs`, `data_device.rs`, etc.).
2. Restructure logic, add/refactor session types, or fix protocol handling.
3. Add doc comments or improve code clarity.

### Fix or Improve Feature and Update Tests
**Trigger:** When a bug is fixed or a feature is improved, especially if the fix is subtle or regression-prone.  
**Command:** `/fix-and-test`

1. Edit the implementation file(s) to fix the bug or improve the feature.
2. Edit or add corresponding test files to cover the change.
3. Commit both implementation and test changes together.

### Docs Minor Fixes and Link Updates
**Trigger:** When documentation errors, typos, or outdated links are found.  
**Command:** `/docs-fix`

1. Edit one or more `docs/**/*.md` files to correct typos or update links.
2. Optionally touch code comments or related doc files.

## Testing Patterns

- **Framework:** Not explicitly specified; likely uses Rust's built-in testing.
- **File Pattern:** Test files typically match `*.test.*` or are located in `term/src/test/*.rs`.
- **Test Example:**
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

      #[test]
      fn test_feature() {
          assert_eq!(my_function(), expected_value);
      }
  }
  ```

## Commands

| Command                     | Purpose                                                    |
|-----------------------------|------------------------------------------------------------|
| /update-ci-workflows        | Regenerate or update CI workflow files                     |
| /update-install-docs        | Add or update installation instructions in documentation   |
| /update-changelog           | Update the changelog after a feature or fix                |
| /nix-flake-update           | Update Nix flake files and related workflows               |
| /wayland-clipboard-refactor | Refactor/fix Wayland clipboard or DnD logic                |
| /fix-and-test               | Fix or improve a feature and update/add tests              |
| /docs-fix                   | Make minor documentation corrections or link updates       |
```
